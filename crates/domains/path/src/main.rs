use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mac-domain-path")]
#[command(about = "PATH 환경변수 선언적 관리 (~/.mac-app-init/paths.json → paths.sh)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// PATH 항목 추가
    Add {
        path: String,
        /// 설명 (선택)
        #[arg(long)]
        label: Option<String>,
    },
    /// PATH 항목 제거
    Rm { path: String },
    /// 항목 on/off 토글
    Toggle { path: String },
    /// 등록된 PATH 목록 (on/off + 실존 여부)
    List,
    /// paths.json → paths.sh 재생성 + zshrc source 보장
    Sync,
    /// 현재 시스템 PATH 에서 자동 스캔 → 등록 제안
    Scan,
    /// 상태 요약
    Status,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Add { path, label } => cmd_add(&path, label.as_deref()),
        Commands::Rm { path } => cmd_rm(&path),
        Commands::Toggle { path } => cmd_toggle(&path),
        Commands::List => cmd_list(),
        Commands::Sync => cmd_sync(),
        Commands::Scan => cmd_scan(),
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

// === 데이터 ===

fn home() -> String { std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()) }

fn paths_json() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/paths.json") }
fn paths_sh() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/paths.sh") }

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
struct PathStore {
    #[serde(default)]
    paths: Vec<PathEntry>,
}

fn load_store() -> PathStore {
    let p = paths_json();
    if !p.exists() { return PathStore::default(); }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save_store(s: &PathStore) {
    let p = paths_json();
    if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&p, serde_json::to_string_pretty(s).unwrap_or_default());
}

fn expand(p: &str) -> String {
    if p.starts_with('~') {
        p.replacen('~', &home(), 1)
    } else { p.to_string() }
}

fn dir_exists(p: &str) -> bool {
    PathBuf::from(expand(p)).is_dir()
}

fn generate_sh(store: &PathStore) {
    let ts = std::process::Command::new("date").args(["+%Y-%m-%d %H:%M:%S"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let mut lines = vec![
        "#!/bin/sh".into(),
        format!("# mac-app-init PATH — 자동 생성 ({}). 직접 수정 금지.", ts),
        "# mac run path add/rm/toggle 로 관리.".into(),
        String::new(),
    ];
    for entry in &store.paths {
        if entry.enabled {
            let comment = if entry.label.is_empty() { String::new() } else { format!("  # {}", entry.label) };
            lines.push(format!("export PATH=\"{}:$PATH\"{}", expand(&entry.path), comment));
        } else {
            let comment = if entry.label.is_empty() { String::new() } else { format!(" # {}", entry.label) };
            lines.push(format!("# [OFF] {}{}", entry.path, comment));
        }
    }
    let sh = paths_sh();
    if let Some(parent) = sh.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&sh, lines.join("\n") + "\n");
}

fn is_sourced_in_zshrc() -> bool {
    let zshrc = PathBuf::from(home()).join(".zshrc");
    fs::read_to_string(&zshrc).unwrap_or_default().contains(".mac-app-init/paths.sh")
}

fn ensure_zshrc_source() {
    if is_sourced_in_zshrc() { return; }
    let zshrc = PathBuf::from(home()).join(".zshrc");
    let mut content = fs::read_to_string(&zshrc).unwrap_or_default();
    if !content.ends_with('\n') { content.push('\n'); }
    content.push_str(&format!("\n# mac-app-init PATH\nsource {}\n", paths_sh().display()));
    let _ = fs::write(&zshrc, content);
}

// === 커맨드 ===

fn cmd_add(path: &str, label: Option<&str>) {
    let mut store = load_store();
    if store.paths.iter().any(|e| e.path == path) {
        println!("이미 등록됨: {}", path);
        return;
    }
    let exists = dir_exists(path);
    store.paths.push(PathEntry {
        path: path.into(),
        enabled: true,
        label: label.unwrap_or("").into(),
    });
    save_store(&store);
    generate_sh(&store);
    ensure_zshrc_source();
    println!("✓ 추가: {}{}", path, if exists { "" } else { " (⚠ 디렉터리 미존재)" });
}

fn cmd_rm(path: &str) {
    let mut store = load_store();
    let before = store.paths.len();
    store.paths.retain(|e| e.path != path);
    if store.paths.len() == before {
        eprintln!("✗ '{}' 없음", path);
        std::process::exit(1);
    }
    save_store(&store);
    generate_sh(&store);
    println!("✓ 제거: {}", path);
}

fn cmd_toggle(path: &str) {
    let mut store = load_store();
    let Some(entry) = store.paths.iter_mut().find(|e| e.path == path) else {
        eprintln!("✗ '{}' 없음", path);
        std::process::exit(1);
    };
    entry.enabled = !entry.enabled;
    let state = if entry.enabled { "ON" } else { "OFF" };
    println!("✓ {} → {}", path, state);
    save_store(&store);
    generate_sh(&store);
}

fn cmd_list() {
    let store = load_store();
    if store.paths.is_empty() {
        println!("등록된 PATH 없음. `mac run path add /opt/homebrew/bin`");
        return;
    }
    println!("{:<6} {:<45} {:<10} {}", "STATE", "PATH", "EXISTS", "LABEL");
    println!("{}", "─".repeat(80));
    for e in &store.paths {
        let state = if e.enabled { "✓ ON" } else { "✗ OFF" };
        let exists = if dir_exists(&e.path) { "✓" } else { "✗" };
        println!("{:<6} {:<45} {:<10} {}", state, e.path, exists, e.label);
    }
}

fn cmd_sync() {
    let store = load_store();
    generate_sh(&store);
    ensure_zshrc_source();
    println!("✓ paths.sh 생성 ({}개 항목, {}개 활성)",
        store.paths.len(),
        store.paths.iter().filter(|e| e.enabled).count());
    println!("✓ ~/.zshrc source {}", if is_sourced_in_zshrc() { "확인됨" } else { "추가됨" });
    println!("\n새 터미널에서 적용됩니다.");
}

fn cmd_scan() {
    let current = std::env::var("PATH").unwrap_or_default();
    let store = load_store();
    let registered: std::collections::HashSet<String> =
        store.paths.iter().map(|e| expand(&e.path)).collect();

    println!("현재 시스템 PATH 에서 미등록 항목:\n");
    let mut suggestions = 0;
    for p in current.split(':') {
        if p.is_empty() { continue; }
        if registered.contains(p) { continue; }
        // 시스템 기본 경로는 제외
        if matches!(p, "/usr/bin" | "/bin" | "/usr/sbin" | "/sbin") { continue; }
        println!("  + {}", p);
        suggestions += 1;
    }
    if suggestions == 0 {
        println!("  (미등록 항목 없음)");
    } else {
        println!("\n등록하려면: mac run path add <경로> --label '설명'");
    }
}

fn cmd_status() {
    let store = load_store();
    let active = store.paths.iter().filter(|e| e.enabled).count();
    let missing = store.paths.iter().filter(|e| e.enabled && !dir_exists(&e.path)).count();
    println!("=== Path Status ===\n");
    println!("등록 {}개 (활성 {}, 비활성 {})", store.paths.len(), active, store.paths.len() - active);
    if missing > 0 {
        println!("⚠ 활성인데 디렉터리 미존재: {}개", missing);
    }
    println!("paths.sh: {}", if paths_sh().exists() { "✓" } else { "✗ (sync 필요)" });
    println!("~/.zshrc source: {}", if is_sourced_in_zshrc() { "✓" } else { "✗ (sync 필요)" });
}

fn print_tui_spec() {
    let store = load_store();
    let items: Vec<serde_json::Value> = store.paths.iter().map(|e| {
        let exists = dir_exists(&e.path);
        let status = if !e.enabled { "warn" } else if exists { "ok" } else { "error" };
        let state = if e.enabled { "ON" } else { "OFF" };
        serde_json::json!({
            "key": e.path,
            "value": format!("{} {}", state, e.label),
            "status": status,
            "data": {
                "name": e.path,
                "path": e.path,
                "enabled": e.enabled.to_string(),
                "exists": exists.to_string(),
                "label": e.label,
            }
        })
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label": "Path", "icon": "🛤" },
        "list_section": "Paths",
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "등록", "value": format!("{}개 (활성 {}개)",
                        store.paths.len(),
                        store.paths.iter().filter(|e| e.enabled).count()),
                      "status": "ok" },
                    { "key": "paths.sh", "value": if paths_sh().exists() { "✓" } else { "✗" },
                      "status": if paths_sh().exists() { "ok" } else { "warn" } },
                    { "key": "~/.zshrc source", "value": if is_sourced_in_zshrc() { "✓" } else { "✗" },
                      "status": if is_sourced_in_zshrc() { "ok" } else { "warn" } },
                ]
            },
            {
                "kind": "key-value",
                "title": "Paths",
                "items": items
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "List", "command": "list", "key": "l" },
                    { "label": "Sync", "command": "sync", "key": "y" },
                    { "label": "Scan", "command": "scan", "key": "c" },
                    { "label": "Status", "command": "status", "key": "s" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "  mac run path add <경로> --label '설명'\n  mac run path toggle <경로>   # on/off\n  mac run path rm <경로>\n  mac run path scan            # 시스템 PATH 에서 미등록 항목 탐색\n  mac run path sync            # paths.sh 재생성 + zshrc 등록"
            }
        ],
        "keybindings": [
            { "key": "T", "label": "on/off 토글",
              "command": "toggle",
              "args": ["${selected.path}"] },
            { "key": "d", "label": "제거",
              "command": "rm",
              "args": ["${selected.path}"],
              "confirm": true }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
