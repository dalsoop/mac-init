use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;
use toml::map::Map as TomlMap;

#[derive(Parser)]
#[command(name = "mac-domain-shell")]
#[command(about = "셸 환경 관리 (PATH + alias + AI 도구 권한 설정)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// PATH 관리
    Path {
        #[command(subcommand)]
        action: PathAction,
    },
    /// Alias 관리
    Alias {
        #[command(subcommand)]
        action: AliasAction,
    },
    /// Codex / Claude Code 권한 설정
    Ai {
        #[command(subcommand)]
        action: AiAction,
    },
    /// shell.sh 재생성 + zshrc source 보장
    Sync,
    /// 통합 상태
    Status,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

#[derive(Subcommand)]
enum PathAction {
    /// PATH 항목 추가
    Add {
        path: String,
        #[arg(long)]
        label: Option<String>,
    },
    /// PATH 항목 제거
    Rm { path: String },
    /// on/off 토글
    Toggle { path: String },
    /// 등록된 PATH 목록
    List,
    /// 시스템 PATH 에서 미등록 항목 탐색
    Scan,
}

#[derive(Subcommand)]
enum AliasAction {
    /// alias 추가 (예: shell alias add mst "mai run mount status")
    Add { name: String, command: String },
    /// alias 제거
    Rm { name: String },
    /// alias 목록
    List,
}

#[derive(Subcommand)]
enum AiAction {
    /// Codex / Claude Code 권한 상태
    Status,
    /// Codex / Claude Code 권한을 최대 허용으로 설정
    Max {
        #[arg(value_enum, default_value_t = AiTarget::All)]
        target: AiTarget,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum AiTarget {
    All,
    Codex,
    Claude,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Path { action } => match action {
            PathAction::Add { path, label } => cmd_path_add(&path, label.as_deref()),
            PathAction::Rm { path } => cmd_path_rm(&path),
            PathAction::Toggle { path } => cmd_path_toggle(&path),
            PathAction::List => cmd_path_list(),
            PathAction::Scan => cmd_path_scan(),
        },
        Commands::Alias { action } => match action {
            AliasAction::Add { name, command } => cmd_alias_add(&name, &command),
            AliasAction::Rm { name } => cmd_alias_rm(&name),
            AliasAction::List => cmd_alias_list(),
        },
        Commands::Ai { action } => match action {
            AiAction::Status => cmd_ai_status(),
            AiAction::Max { target } => cmd_ai_max(target),
        },
        Commands::Sync => cmd_sync(),
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

// === 데이터 모델 ===

use mac_common::{
    paths,
    tui_spec::{self, TuiSpec},
};

fn home() -> String {
    paths::home()
}
fn store_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/shell.json")
}
fn shell_sh() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/shell.sh")
}
fn codex_config_path() -> PathBuf {
    PathBuf::from(home()).join(".codex/config.toml")
}
fn claude_settings_path() -> PathBuf {
    PathBuf::from(home()).join(".claude/settings.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PathEntry {
    path: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    label: String,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ShellStore {
    #[serde(default)]
    paths: Vec<PathEntry>,
    #[serde(default)]
    aliases: BTreeMap<String, String>,
}

fn load() -> ShellStore {
    let p = store_path();
    if !p.exists() {
        return ShellStore::default();
    }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save(s: &ShellStore) {
    let p = store_path();
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&p, serde_json::to_string_pretty(s).unwrap_or_default());
}

fn dir_exists(p: &str) -> bool {
    PathBuf::from(paths::expand(p)).is_dir()
}

fn now_str() -> String {
    std::process::Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn generate_sh(s: &ShellStore) {
    let mut lines = vec![
        "#!/bin/sh".into(),
        format!(
            "# mac-app-init shell — 자동 생성 ({}). 직접 수정 금지.",
            now_str()
        ),
        "# mai run shell path/alias 로 관리.".into(),
        String::new(),
        "# === PATH ===".into(),
    ];
    for e in &s.paths {
        if e.enabled {
            let c = if e.label.is_empty() {
                String::new()
            } else {
                format!("  # {}", e.label)
            };
            lines.push(format!(
                "export PATH=\"{}:$PATH\"{}",
                paths::expand(&e.path),
                c
            ));
        } else {
            let c = if e.label.is_empty() {
                String::new()
            } else {
                format!(" # {}", e.label)
            };
            lines.push(format!("# [OFF] {}{}", e.path, c));
        }
    }
    lines.push(String::new());
    lines.push("# === Aliases ===".into());
    for (name, cmd) in &s.aliases {
        lines.push(format!("alias {}='{}'", name, cmd.replace('\'', "'\\''")));
    }
    let sh = shell_sh();
    if let Some(parent) = sh.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&sh, lines.join("\n") + "\n");
}

fn is_sourced() -> bool {
    let zshrc = PathBuf::from(home()).join(".zshrc");
    fs::read_to_string(&zshrc)
        .unwrap_or_default()
        .contains(".mac-app-init/shell.sh")
}

fn ensure_source() {
    if is_sourced() {
        return;
    }
    let zshrc = PathBuf::from(home()).join(".zshrc");
    let mut content = fs::read_to_string(&zshrc).unwrap_or_default();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "\n# mac-app-init shell\nsource {}\n",
        shell_sh().display()
    ));
    let _ = fs::write(&zshrc, content);
}

fn apply(s: &ShellStore) {
    save(s);
    generate_sh(s);
    ensure_source();
}

#[derive(Debug, Default)]
struct CodexConfigStatus {
    exists: bool,
    approval_policy: String,
    sandbox_mode: String,
    web_search: String,
}

#[derive(Debug, Default)]
struct ClaudeConfigStatus {
    exists: bool,
    default_mode: String,
    skip_danger_prompt: bool,
    sandbox_enabled: bool,
    allow_unsandboxed_commands: bool,
    allow_count: usize,
}

fn load_toml_doc(path: &Path) -> Result<TomlValue, String> {
    if !path.exists() {
        return Ok(TomlValue::Table(TomlMap::new()));
    }
    let content =
        fs::read_to_string(path).map_err(|err| format!("{} 읽기 실패: {err}", path.display()))?;
    toml::from_str(&content).map_err(|err| format!("{} TOML 파싱 실패: {err}", path.display()))
}

fn save_toml_doc(path: &Path, value: &TomlValue) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("{} 디렉터리 생성 실패: {err}", parent.display()))?;
    }
    let content = toml::to_string_pretty(value)
        .map_err(|err| format!("{} TOML 직렬화 실패: {err}", path.display()))?;
    fs::write(path, content).map_err(|err| format!("{} 저장 실패: {err}", path.display()))
}

fn load_json_doc(path: &Path) -> Result<JsonValue, String> {
    if !path.exists() {
        return Ok(JsonValue::Object(JsonMap::new()));
    }
    let content =
        fs::read_to_string(path).map_err(|err| format!("{} 읽기 실패: {err}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|err| format!("{} JSON 파싱 실패: {err}", path.display()))
}

fn save_json_doc(path: &Path, value: &JsonValue) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("{} 디렉터리 생성 실패: {err}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(value)
        .map_err(|err| format!("{} JSON 직렬화 실패: {err}", path.display()))?;
    fs::write(path, content).map_err(|err| format!("{} 저장 실패: {err}", path.display()))
}

fn toml_root_mut(value: &mut TomlValue) -> &mut TomlMap<String, TomlValue> {
    if !value.is_table() {
        *value = TomlValue::Table(TomlMap::new());
    }
    value.as_table_mut().expect("root table")
}

fn json_object_mut(value: &mut JsonValue) -> &mut JsonMap<String, JsonValue> {
    if !value.is_object() {
        *value = JsonValue::Object(JsonMap::new());
    }
    value.as_object_mut().expect("root object")
}

fn json_child_object_mut<'a>(
    parent: &'a mut JsonMap<String, JsonValue>,
    key: &str,
) -> &'a mut JsonMap<String, JsonValue> {
    let needs_init = !parent.get(key).map(|v| v.is_object()).unwrap_or(false);
    if needs_init {
        parent.insert(key.to_string(), JsonValue::Object(JsonMap::new()));
    }
    parent
        .get_mut(key)
        .and_then(JsonValue::as_object_mut)
        .expect("child object")
}

fn read_codex_status() -> Result<CodexConfigStatus, String> {
    let path = codex_config_path();
    let value = load_toml_doc(&path)?;
    let root = value.as_table();
    Ok(CodexConfigStatus {
        exists: path.exists(),
        approval_policy: root
            .and_then(|t| t.get("approval_policy"))
            .and_then(TomlValue::as_str)
            .unwrap_or_default()
            .to_string(),
        sandbox_mode: root
            .and_then(|t| t.get("sandbox_mode"))
            .and_then(TomlValue::as_str)
            .unwrap_or_default()
            .to_string(),
        web_search: root
            .and_then(|t| t.get("web_search"))
            .and_then(TomlValue::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn read_claude_status() -> Result<ClaudeConfigStatus, String> {
    let path = claude_settings_path();
    let value = load_json_doc(&path)?;
    let permissions = value.get("permissions").and_then(JsonValue::as_object);
    let sandbox = value.get("sandbox").and_then(JsonValue::as_object);
    let allow_count = permissions
        .and_then(|p| p.get("allow"))
        .and_then(JsonValue::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    Ok(ClaudeConfigStatus {
        exists: path.exists(),
        default_mode: permissions
            .and_then(|p| p.get("defaultMode"))
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string(),
        skip_danger_prompt: permissions
            .and_then(|p| p.get("skipDangerousModePermissionPrompt"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false),
        sandbox_enabled: sandbox
            .and_then(|s| s.get("enabled"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false),
        allow_unsandboxed_commands: sandbox
            .and_then(|s| s.get("allowUnsandboxedCommands"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(true),
        allow_count,
    })
}

fn codex_is_max(status: &CodexConfigStatus) -> bool {
    status.approval_policy == "never"
        && status.sandbox_mode == "danger-full-access"
        && status.web_search == "live"
}

fn claude_is_max(status: &ClaudeConfigStatus) -> bool {
    status.default_mode == "bypassPermissions"
        && status.skip_danger_prompt
        && !status.sandbox_enabled
        && status.allow_unsandboxed_commands
}

fn set_codex_max() -> Result<(), String> {
    let path = codex_config_path();
    let mut value = load_toml_doc(&path)?;
    let root = toml_root_mut(&mut value);
    root.insert(
        "approval_policy".to_string(),
        TomlValue::String("never".to_string()),
    );
    root.insert(
        "sandbox_mode".to_string(),
        TomlValue::String("danger-full-access".to_string()),
    );
    root.insert(
        "web_search".to_string(),
        TomlValue::String("live".to_string()),
    );
    save_toml_doc(&path, &value)
}

fn set_claude_max() -> Result<(), String> {
    let path = claude_settings_path();
    let mut value = load_json_doc(&path)?;
    let root = json_object_mut(&mut value);
    root.entry("$schema".to_string()).or_insert_with(|| {
        JsonValue::String("https://json.schemastore.org/claude-code-settings.json".to_string())
    });
    let permissions = json_child_object_mut(root, "permissions");
    permissions.insert(
        "allow".to_string(),
        serde_json::json!([
            "Bash(*)",
            "Edit(*)",
            "Write(*)",
            "NotebookEdit(*)",
            "Read(*)",
            "WebFetch(*)",
            "MCP(*)"
        ]),
    );
    permissions.insert("ask".to_string(), serde_json::json!([]));
    permissions.insert("deny".to_string(), serde_json::json!([]));
    permissions.insert(
        "defaultMode".to_string(),
        JsonValue::String("bypassPermissions".to_string()),
    );
    permissions.insert(
        "skipDangerousModePermissionPrompt".to_string(),
        JsonValue::Bool(true),
    );

    let sandbox = json_child_object_mut(root, "sandbox");
    sandbox.insert("enabled".to_string(), JsonValue::Bool(false));
    sandbox.insert(
        "autoAllowBashIfSandboxed".to_string(),
        JsonValue::Bool(true),
    );
    sandbox.insert(
        "allowUnsandboxedCommands".to_string(),
        JsonValue::Bool(true),
    );

    save_json_doc(&path, &value)
}

fn print_ai_status_summary() {
    match read_codex_status() {
        Ok(status) => {
            println!(
                "[Codex] {} ({})",
                if status.exists {
                    "✓ 설정 파일"
                } else {
                    "✗ 설정 파일 없음"
                },
                codex_config_path().display()
            );
            println!(
                "  approval_policy={} sandbox_mode={} web_search={} [{}]",
                if status.approval_policy.is_empty() {
                    "(unset)"
                } else {
                    &status.approval_policy
                },
                if status.sandbox_mode.is_empty() {
                    "(unset)"
                } else {
                    &status.sandbox_mode
                },
                if status.web_search.is_empty() {
                    "(unset)"
                } else {
                    &status.web_search
                },
                if codex_is_max(&status) {
                    "MAX"
                } else {
                    "기본/사용자설정"
                }
            );
        }
        Err(err) => {
            println!("[Codex] ✗ {err}");
        }
    }

    match read_claude_status() {
        Ok(status) => {
            println!(
                "[Claude Code] {} ({})",
                if status.exists {
                    "✓ 설정 파일"
                } else {
                    "✗ 설정 파일 없음"
                },
                claude_settings_path().display()
            );
            println!(
                "  defaultMode={} skipDangerPrompt={} sandbox.enabled={} allowUnsandboxed={} allow={}개 [{}]",
                if status.default_mode.is_empty() {
                    "(unset)"
                } else {
                    &status.default_mode
                },
                if status.skip_danger_prompt {
                    "true"
                } else {
                    "false"
                },
                if status.sandbox_enabled {
                    "true"
                } else {
                    "false"
                },
                if status.allow_unsandboxed_commands {
                    "true"
                } else {
                    "false"
                },
                status.allow_count,
                if claude_is_max(&status) {
                    "MAX"
                } else {
                    "기본/사용자설정"
                }
            );
        }
        Err(err) => {
            println!("[Claude Code] ✗ {err}");
        }
    }
}

fn cmd_ai_status() {
    println!("=== AI 권한 상태 ===\n");
    print_ai_status_summary();
    println!("\n설정 명령:");
    println!("  mai run shell ai max codex");
    println!("  mai run shell ai max claude");
    println!("  mai run shell ai max");
}

fn cmd_ai_max(target: AiTarget) {
    println!("=== AI 권한 최대 허용 설정 ===\n");

    match target {
        AiTarget::All | AiTarget::Codex => match set_codex_max() {
            Ok(()) => {
                println!("[Codex] 최대 허용 설정 완료");
                println!("  approval_policy=never");
                println!("  sandbox_mode=danger-full-access");
                println!("  web_search=live");
                println!("  파일: {}", codex_config_path().display());
            }
            Err(err) => {
                eprintln!("[Codex] 설정 실패: {err}");
                std::process::exit(1);
            }
        },
        AiTarget::Claude => {}
    }

    match target {
        AiTarget::All | AiTarget::Claude => match set_claude_max() {
            Ok(()) => {
                println!("[Claude Code] 최대 허용 설정 완료");
                println!("  permissions.defaultMode=bypassPermissions");
                println!("  permissions.skipDangerousModePermissionPrompt=true");
                println!("  sandbox.enabled=false");
                println!("  sandbox.allowUnsandboxedCommands=true");
                println!("  파일: {}", claude_settings_path().display());
            }
            Err(err) => {
                eprintln!("[Claude Code] 설정 실패: {err}");
                std::process::exit(1);
            }
        },
        AiTarget::Codex => {}
    }

    println!("\n현재 상태:");
    print_ai_status_summary();
}

// === PATH 커맨드 ===

fn cmd_path_add(path: &str, label: Option<&str>) {
    let mut s = load();
    if s.paths.iter().any(|e| e.path == path) {
        println!("이미 등록됨: {}", path);
        return;
    }
    let exists = dir_exists(path);
    s.paths.push(PathEntry {
        path: path.into(),
        enabled: true,
        label: label.unwrap_or("").into(),
    });
    apply(&s);
    println!(
        "✓ path 추가: {}{}",
        path,
        if exists {
            ""
        } else {
            " (⚠ 디렉터리 미존재)"
        }
    );
}

fn cmd_path_rm(path: &str) {
    let mut s = load();
    let before = s.paths.len();
    s.paths.retain(|e| e.path != path);
    if s.paths.len() == before {
        eprintln!("✗ '{}' 없음", path);
        std::process::exit(1);
    }
    apply(&s);
    println!("✓ path 제거: {}", path);
}

fn cmd_path_toggle(path: &str) {
    let mut s = load();
    let Some(e) = s.paths.iter_mut().find(|e| e.path == path) else {
        eprintln!("✗ '{}' 없음", path);
        std::process::exit(1);
    };
    e.enabled = !e.enabled;
    let state = if e.enabled { "ON" } else { "OFF" };
    apply(&s);
    println!("✓ {} → {}", path, state);
}

fn cmd_path_list() {
    let s = load();
    if s.paths.is_empty() {
        println!("등록된 PATH 없음.");
        return;
    }
    println!("{:<6} {:<45} {:<6} {}", "STATE", "PATH", "DIR", "LABEL");
    println!("{}", "─".repeat(75));
    for e in &s.paths {
        let state = if e.enabled { "✓ ON" } else { "✗ OFF" };
        let exists = if dir_exists(&e.path) { "✓" } else { "✗" };
        println!("{:<6} {:<45} {:<6} {}", state, e.path, exists, e.label);
    }
}

fn cmd_path_scan() {
    let current = std::env::var("PATH").unwrap_or_default();
    let s = load();
    let registered: std::collections::HashSet<String> =
        s.paths.iter().map(|e| paths::expand(&e.path)).collect();
    let skip = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];
    let mut seen = std::collections::HashSet::new();
    println!("시스템 PATH 미등록 항목:\n");
    let mut count = 0;
    for p in current.split(':') {
        if p.is_empty() || skip.contains(&p) {
            continue;
        }
        if registered.contains(p) || !seen.insert(p.to_string()) {
            continue;
        }
        println!("  + {}", p);
        count += 1;
    }
    if count == 0 {
        println!("  (없음)");
    } else {
        println!("\n등록: mai run shell path add <경로> --label '설명'");
    }
}

// === Alias 커맨드 ===

fn cmd_alias_add(name: &str, command: &str) {
    let mut s = load();
    let existed = s.aliases.contains_key(name);
    s.aliases.insert(name.into(), command.into());
    apply(&s);
    println!(
        "✓ alias {}: {} → '{}'",
        if existed { "갱신" } else { "추가" },
        name,
        command
    );
}

fn cmd_alias_rm(name: &str) {
    let mut s = load();
    if s.aliases.remove(name).is_none() {
        eprintln!("✗ '{}' 없음", name);
        std::process::exit(1);
    }
    apply(&s);
    println!("✓ alias 제거: {}", name);
}

fn cmd_alias_list() {
    let s = load();
    if s.aliases.is_empty() {
        println!("등록된 alias 없음.");
        return;
    }
    println!("{:<20} {}", "ALIAS", "COMMAND");
    println!("{}", "─".repeat(50));
    for (name, cmd) in &s.aliases {
        println!("{:<20} {}", name, cmd);
    }
}

// === 통합 ===

fn cmd_sync() {
    let s = load();
    generate_sh(&s);
    ensure_source();
    println!(
        "✓ shell.sh 생성 (PATH {}개, alias {}개)",
        s.paths.len(),
        s.aliases.len()
    );
    println!(
        "✓ ~/.zshrc source {}",
        if is_sourced() {
            "확인됨"
        } else {
            "추가됨"
        }
    );
    println!("\n새 터미널에서 적용.");
}

fn cmd_status() {
    let s = load();
    let active = s.paths.iter().filter(|e| e.enabled).count();
    let missing = s
        .paths
        .iter()
        .filter(|e| e.enabled && !dir_exists(&e.path))
        .count();
    println!("=== Shell Status ===\n");
    println!(
        "PATH  : {}개 (활성 {}, 비활성 {})",
        s.paths.len(),
        active,
        s.paths.len() - active
    );
    if missing > 0 {
        println!("  ⚠ 활성인데 디렉터리 미존재: {}개", missing);
    }
    println!("alias : {}개", s.aliases.len());
    println!(
        "shell.sh: {}",
        if shell_sh().exists() {
            "✓"
        } else {
            "✗ (sync 필요)"
        }
    );
    println!(
        "~/.zshrc: {}",
        if is_sourced() {
            "✓ source"
        } else {
            "✗ (sync 필요)"
        }
    );
    println!();
    print_ai_status_summary();
}

fn print_tui_spec() {
    let s = load();
    let path_items: Vec<serde_json::Value> = s.paths.iter().map(|e| {
        let exists = dir_exists(&e.path);
        let status = if !e.enabled { "warn" } else if exists { "ok" } else { "error" };
        tui_spec::kv_item_data(&e.path,
            &format!("{} {}", if e.enabled {"ON"} else {"OFF"}, e.label),
            status,
            serde_json::json!({ "name": e.path, "path": e.path, "enabled": e.enabled.to_string(), "label": e.label }))
    }).collect();

    let alias_items: Vec<serde_json::Value> = s
        .aliases
        .iter()
        .map(|(name, cmd_str)| {
            tui_spec::kv_item_data(
                name,
                cmd_str,
                "ok",
                serde_json::json!({ "name": name, "alias_name": name, "command": cmd_str }),
            )
        })
        .collect();

    let active_paths = s.paths.iter().filter(|e| e.enabled).count();
    let usage_active = active_paths > 0 || !s.aliases.is_empty();
    let codex_status = read_codex_status().ok();
    let claude_status = read_claude_status().ok();
    let usage_summary = format!(
        "PATH {}개, alias {}개, Codex {}, Claude {}",
        active_paths,
        s.aliases.len(),
        if codex_status.as_ref().map(codex_is_max).unwrap_or(false) {
            "MAX"
        } else {
            "기본"
        },
        if claude_status.as_ref().map(claude_is_max).unwrap_or(false) {
            "MAX"
        } else {
            "기본"
        }
    );

    let ai_items = vec![
        match codex_status {
            Some(status) => tui_spec::kv_item(
                "Codex",
                &format!(
                    "{} / {} / {}",
                    if status.approval_policy.is_empty() {
                        "(unset)"
                    } else {
                        &status.approval_policy
                    },
                    if status.sandbox_mode.is_empty() {
                        "(unset)"
                    } else {
                        &status.sandbox_mode
                    },
                    if status.web_search.is_empty() {
                        "(unset)"
                    } else {
                        &status.web_search
                    }
                ),
                if codex_is_max(&status) { "ok" } else { "warn" },
            ),
            None => tui_spec::kv_item("Codex", "config parse error", "error"),
        },
        match claude_status {
            Some(status) => tui_spec::kv_item(
                "Claude Code",
                &format!(
                    "{} / prompt={} / sandbox={}",
                    if status.default_mode.is_empty() {
                        "(unset)"
                    } else {
                        &status.default_mode
                    },
                    if status.skip_danger_prompt {
                        "skip"
                    } else {
                        "ask"
                    },
                    if status.sandbox_enabled { "on" } else { "off" }
                ),
                if claude_is_max(&status) { "ok" } else { "warn" },
            ),
            None => tui_spec::kv_item("Claude Code", "settings parse error", "error"),
        },
    ];

    TuiSpec::new("shell")
        .list_section("PATH")
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("PATH",
                &format!("{}개 (활성 {})", s.paths.len(), s.paths.iter().filter(|e|e.enabled).count()), "ok"),
            tui_spec::kv_item("alias", &format!("{}개", s.aliases.len()), "ok"),
            tui_spec::kv_item("shell.sh",
                if shell_sh().exists() {"✓"} else {"✗"},
                if shell_sh().exists() {"ok"} else {"warn"}),
            tui_spec::kv_item("~/.zshrc",
                if is_sourced() {"✓ source"} else {"✗"},
                if is_sourced() {"ok"} else {"warn"}),
        ])
        .kv("AI 권한", ai_items)
        .kv("PATH", path_items)
        .kv("별칭", alias_items)
        .buttons()
        .buttons_custom("AI 실행", vec![
            serde_json::json!({
                "label": "AI 권한 상태",
                "command": "ai",
                "args": ["status"],
                "key": "i"
            }),
            serde_json::json!({
                "label": "Codex 최대 허용",
                "command": "ai",
                "args": ["max", "codex"],
                "key": "c"
            }),
            serde_json::json!({
                "label": "Claude 최대 허용",
                "command": "ai",
                "args": ["max", "claude"],
                "key": "l"
            }),
            serde_json::json!({
                "label": "둘 다 최대 허용",
                "command": "ai",
                "args": ["max"],
                "key": "m"
            }),
        ])
        .text("안내", "  mai run shell path add <경로> --label '설명'\n  mai run shell path toggle <경로>\n  mai run shell alias add <name> <command>\n  mai run shell sync")
        .print();
}
