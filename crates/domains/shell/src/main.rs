use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mac-domain-shell")]
#[command(about = "셸 환경 관리 (PATH + alias → shell.sh)")]
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
    /// alias 추가 (예: shell alias add mst "mac run mount status")
    Add { name: String, command: String },
    /// alias 제거
    Rm { name: String },
    /// alias 목록
    List,
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
        Commands::Sync => cmd_sync(),
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

// === 데이터 모델 ===

fn home() -> String { std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()) }
fn store_path() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/shell.json") }
fn shell_sh() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/shell.sh") }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PathEntry {
    path: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    label: String,
}
fn default_true() -> bool { true }

#[derive(Debug, Default, Serialize, Deserialize)]
struct ShellStore {
    #[serde(default)]
    paths: Vec<PathEntry>,
    #[serde(default)]
    aliases: BTreeMap<String, String>,
}

fn load() -> ShellStore {
    let p = store_path();
    if !p.exists() { return ShellStore::default(); }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save(s: &ShellStore) {
    let p = store_path();
    if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&p, serde_json::to_string_pretty(s).unwrap_or_default());
}

fn expand(p: &str) -> String {
    if p.starts_with('~') { p.replacen('~', &home(), 1) } else { p.to_string() }
}

fn dir_exists(p: &str) -> bool { PathBuf::from(expand(p)).is_dir() }

fn now_str() -> String {
    std::process::Command::new("date").args(["+%Y-%m-%d %H:%M:%S"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn generate_sh(s: &ShellStore) {
    let mut lines = vec![
        "#!/bin/sh".into(),
        format!("# mac-app-init shell — 자동 생성 ({}). 직접 수정 금지.", now_str()),
        "# mac run shell path/alias 로 관리.".into(),
        String::new(),
        "# === PATH ===".into(),
    ];
    for e in &s.paths {
        if e.enabled {
            let c = if e.label.is_empty() { String::new() } else { format!("  # {}", e.label) };
            lines.push(format!("export PATH=\"{}:$PATH\"{}", expand(&e.path), c));
        } else {
            let c = if e.label.is_empty() { String::new() } else { format!(" # {}", e.label) };
            lines.push(format!("# [OFF] {}{}", e.path, c));
        }
    }
    lines.push(String::new());
    lines.push("# === Aliases ===".into());
    for (name, cmd) in &s.aliases {
        lines.push(format!("alias {}='{}'", name, cmd.replace('\'', "'\\''")));
    }
    let sh = shell_sh();
    if let Some(parent) = sh.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&sh, lines.join("\n") + "\n");
}

fn is_sourced() -> bool {
    let zshrc = PathBuf::from(home()).join(".zshrc");
    fs::read_to_string(&zshrc).unwrap_or_default().contains(".mac-app-init/shell.sh")
}

fn ensure_source() {
    if is_sourced() { return; }
    let zshrc = PathBuf::from(home()).join(".zshrc");
    let mut content = fs::read_to_string(&zshrc).unwrap_or_default();
    if !content.ends_with('\n') { content.push('\n'); }
    content.push_str(&format!("\n# mac-app-init shell\nsource {}\n", shell_sh().display()));
    let _ = fs::write(&zshrc, content);
}

fn apply(s: &ShellStore) {
    save(s);
    generate_sh(s);
    ensure_source();
}

// === PATH 커맨드 ===

fn cmd_path_add(path: &str, label: Option<&str>) {
    let mut s = load();
    if s.paths.iter().any(|e| e.path == path) {
        println!("이미 등록됨: {}", path);
        return;
    }
    let exists = dir_exists(path);
    s.paths.push(PathEntry { path: path.into(), enabled: true, label: label.unwrap_or("").into() });
    apply(&s);
    println!("✓ path 추가: {}{}", path, if exists { "" } else { " (⚠ 디렉터리 미존재)" });
}

fn cmd_path_rm(path: &str) {
    let mut s = load();
    let before = s.paths.len();
    s.paths.retain(|e| e.path != path);
    if s.paths.len() == before { eprintln!("✗ '{}' 없음", path); std::process::exit(1); }
    apply(&s);
    println!("✓ path 제거: {}", path);
}

fn cmd_path_toggle(path: &str) {
    let mut s = load();
    let Some(e) = s.paths.iter_mut().find(|e| e.path == path) else {
        eprintln!("✗ '{}' 없음", path); std::process::exit(1);
    };
    e.enabled = !e.enabled;
    let state = if e.enabled { "ON" } else { "OFF" };
    apply(&s);
    println!("✓ {} → {}", path, state);
}

fn cmd_path_list() {
    let s = load();
    if s.paths.is_empty() { println!("등록된 PATH 없음."); return; }
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
    let registered: std::collections::HashSet<String> = s.paths.iter().map(|e| expand(&e.path)).collect();
    let skip = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];
    let mut seen = std::collections::HashSet::new();
    println!("시스템 PATH 미등록 항목:\n");
    let mut count = 0;
    for p in current.split(':') {
        if p.is_empty() || skip.contains(&p) { continue; }
        if registered.contains(p) || !seen.insert(p.to_string()) { continue; }
        println!("  + {}", p);
        count += 1;
    }
    if count == 0 { println!("  (없음)"); }
    else { println!("\n등록: mac run shell path add <경로> --label '설명'"); }
}

// === Alias 커맨드 ===

fn cmd_alias_add(name: &str, command: &str) {
    let mut s = load();
    let existed = s.aliases.contains_key(name);
    s.aliases.insert(name.into(), command.into());
    apply(&s);
    println!("✓ alias {}: {} → '{}'", if existed { "갱신" } else { "추가" }, name, command);
}

fn cmd_alias_rm(name: &str) {
    let mut s = load();
    if s.aliases.remove(name).is_none() { eprintln!("✗ '{}' 없음", name); std::process::exit(1); }
    apply(&s);
    println!("✓ alias 제거: {}", name);
}

fn cmd_alias_list() {
    let s = load();
    if s.aliases.is_empty() { println!("등록된 alias 없음."); return; }
    println!("{:<20} {}", "ALIAS", "COMMAND");
    println!("{}", "─".repeat(50));
    for (name, cmd) in &s.aliases { println!("{:<20} {}", name, cmd); }
}

// === 통합 ===

fn cmd_sync() {
    let s = load();
    generate_sh(&s);
    ensure_source();
    println!("✓ shell.sh 생성 (PATH {}개, alias {}개)", s.paths.len(), s.aliases.len());
    println!("✓ ~/.zshrc source {}", if is_sourced() { "확인됨" } else { "추가됨" });
    println!("\n새 터미널에서 적용.");
}

fn cmd_status() {
    let s = load();
    let active = s.paths.iter().filter(|e| e.enabled).count();
    let missing = s.paths.iter().filter(|e| e.enabled && !dir_exists(&e.path)).count();
    println!("=== Shell Status ===\n");
    println!("PATH  : {}개 (활성 {}, 비활성 {})", s.paths.len(), active, s.paths.len() - active);
    if missing > 0 { println!("  ⚠ 활성인데 디렉터리 미존재: {}개", missing); }
    println!("alias : {}개", s.aliases.len());
    println!("shell.sh: {}", if shell_sh().exists() { "✓" } else { "✗ (sync 필요)" });
    println!("~/.zshrc: {}", if is_sourced() { "✓ source" } else { "✗ (sync 필요)" });
}

fn print_tui_spec() {
    let s = load();
    let path_items: Vec<serde_json::Value> = s.paths.iter().map(|e| {
        let exists = dir_exists(&e.path);
        let status = if !e.enabled { "warn" } else if exists { "ok" } else { "error" };
        serde_json::json!({
            "key": e.path, "value": format!("{} {}", if e.enabled {"ON"} else {"OFF"}, e.label),
            "status": status,
            "data": { "name": e.path, "path": e.path, "enabled": e.enabled.to_string(), "label": e.label }
        })
    }).collect();

    let alias_items: Vec<serde_json::Value> = s.aliases.iter().map(|(name, cmd)| {
        serde_json::json!({
            "key": name, "value": cmd, "status": "ok",
            "data": { "name": name, "alias_name": name, "command": cmd }
        })
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label": "Shell", "icon": "🐚" },
        "list_section": "PATH",
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "PATH", "value": format!("{}개 (활성 {})", s.paths.len(), s.paths.iter().filter(|e|e.enabled).count()), "status": "ok" },
                    { "key": "alias", "value": format!("{}개", s.aliases.len()), "status": "ok" },
                    { "key": "shell.sh", "value": if shell_sh().exists() {"✓"} else {"✗"}, "status": if shell_sh().exists() {"ok"} else {"warn"} },
                    { "key": "~/.zshrc", "value": if is_sourced() {"✓ source"} else {"✗"}, "status": if is_sourced() {"ok"} else {"warn"} },
                ]
            },
            { "kind": "key-value", "title": "PATH", "items": path_items },
            { "kind": "key-value", "title": "Aliases", "items": alias_items },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Path list", "command": "path list", "key": "l" },
                    { "label": "Alias list", "command": "alias list", "key": "a" },
                    { "label": "Scan", "command": "path scan", "key": "c" },
                    { "label": "Sync", "command": "sync", "key": "y" },
                    { "label": "Status", "command": "status", "key": "s" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "  mac run shell path add <경로> --label '설명'\n  mac run shell path toggle <경로>\n  mac run shell alias add <name> <command>\n  mac run shell sync"
            }
        ],
        "keybindings": [
            { "key": "T", "label": "PATH on/off", "command": "path toggle", "args": ["${selected.path}"] },
            { "key": "d", "label": "제거", "command": "path rm", "args": ["${selected.path}"], "confirm": true }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
