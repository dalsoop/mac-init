use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mac-domain-alias")]
#[command(about = "사용자 커스텀 alias 관리 (~/.mac-app-init/aliases.sh)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// alias 추가 (예: alias add mt mac-tui)
    Add { name: String, command: String },
    /// alias 제거
    Rm { name: String },
    /// alias 목록
    List,
    /// aliases.sh 재생성 (JSON → sh 동기화)
    Sync,
    /// 상태 확인 (등록된 alias 수 + shell source 상태)
    Status,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Add { name, command } => cmd_add(&name, &command),
        Commands::Rm { name } => cmd_rm(&name),
        Commands::List => cmd_list(),
        Commands::Sync => cmd_sync(),
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn home() -> String { std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()) }

fn aliases_json_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/aliases.json")
}

fn aliases_sh_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/aliases.sh")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AliasStore {
    #[serde(default)]
    aliases: BTreeMap<String, String>,
}

fn load_store() -> AliasStore {
    let p = aliases_json_path();
    if !p.exists() { return AliasStore::default(); }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save_store(s: &AliasStore) {
    let p = aliases_json_path();
    if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&p, serde_json::to_string_pretty(s).unwrap_or_default());
}

fn generate_sh(s: &AliasStore) {
    let mut lines = vec![
        "# mac-app-init aliases — 자동 생성. 직접 수정 금지.".into(),
        "# mac run alias add/rm 으로 관리.".into(),
        format!("# 생성 시각: {}", chrono_now()),
        String::new(),
        "# PATH: 도메인 바이너리".into(),
        format!("export PATH=\"{}/{}:$PATH\"",
            home(), ".mac-app-init/domains"),
        String::new(),
    ];
    for (name, cmd) in &s.aliases {
        lines.push(format!("alias {}='{}'", name, cmd.replace('\'', "'\\''")));
    }
    let sh = aliases_sh_path();
    if let Some(parent) = sh.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&sh, lines.join("\n") + "\n");
}

fn chrono_now() -> String {
    std::process::Command::new("date").args(["+%Y-%m-%d %H:%M:%S"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn is_sourced() -> bool {
    let zshrc = PathBuf::from(home()).join(".zshrc");
    if !zshrc.exists() { return false; }
    let content = fs::read_to_string(&zshrc).unwrap_or_default();
    content.contains(".mac-app-init/aliases.sh")
}

fn cmd_add(name: &str, command: &str) {
    let mut store = load_store();
    let existed = store.aliases.contains_key(name);
    store.aliases.insert(name.into(), command.into());
    save_store(&store);
    generate_sh(&store);
    if existed {
        println!("✓ 갱신: {} → '{}'", name, command);
    } else {
        println!("✓ 추가: {} → '{}'", name, command);
    }
    if !is_sourced() {
        eprintln!("⚠ ~/.zshrc 에 source 줄 없음. `mac run alias sync` 또는 아래 실행:");
        eprintln!("  echo 'source ~/.mac-app-init/aliases.sh' >> ~/.zshrc");
    } else {
        eprintln!("  새 터미널 열면 적용됩니다.");
    }
}

fn cmd_rm(name: &str) {
    let mut store = load_store();
    if store.aliases.remove(name).is_none() {
        eprintln!("✗ '{}' alias 없음", name);
        std::process::exit(1);
    }
    save_store(&store);
    generate_sh(&store);
    println!("✓ 제거: {}", name);
}

fn cmd_list() {
    let store = load_store();
    if store.aliases.is_empty() {
        println!("등록된 alias 없음. `mac run alias add mt mac-tui`");
        return;
    }
    println!("{:<20} {}", "ALIAS", "COMMAND");
    println!("{}", "─".repeat(50));
    for (name, cmd) in &store.aliases {
        println!("{:<20} {}", name, cmd);
    }
}

fn cmd_sync() {
    let store = load_store();
    generate_sh(&store);
    println!("✓ aliases.sh 생성 ({}개 alias)", store.aliases.len());

    let zshrc = PathBuf::from(home()).join(".zshrc");
    let source_line = format!("source {}", aliases_sh_path().display());
    if !is_sourced() {
        let mut content = fs::read_to_string(&zshrc).unwrap_or_default();
        if !content.ends_with('\n') { content.push('\n'); }
        content.push_str(&format!("\n# mac-app-init aliases\n{}\n", source_line));
        let _ = fs::write(&zshrc, content);
        println!("✓ ~/.zshrc 에 source 줄 추가됨");
    } else {
        println!("✓ ~/.zshrc 에 source 줄 이미 있음");
    }
}

fn cmd_status() {
    let store = load_store();
    println!("=== Alias Status ===\n");
    println!("alias {}개", store.aliases.len());
    println!("aliases.sh: {}", if aliases_sh_path().exists() { "✓ 존재" } else { "✗ 없음 (sync 필요)" });
    println!("~/.zshrc source: {}", if is_sourced() { "✓" } else { "✗ (sync 필요)" });
    println!("PATH 포함: ~/.mac-app-init/domains/");
}

fn print_tui_spec() {
    let store = load_store();
    let items: Vec<serde_json::Value> = store.aliases.iter().map(|(name, cmd)| {
        serde_json::json!({
            "key": name,
            "value": cmd,
            "status": "ok",
            "data": { "name": name, "command": cmd }
        })
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label": "Alias", "icon": "🔗" },
        "list_section": "Aliases",
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "등록 alias", "value": format!("{}개", store.aliases.len()), "status": "ok" },
                    { "key": "aliases.sh", "value": if aliases_sh_path().exists() { "✓" } else { "✗" },
                      "status": if aliases_sh_path().exists() { "ok" } else { "warn" } },
                    { "key": "~/.zshrc source", "value": if is_sourced() { "✓" } else { "✗" },
                      "status": if is_sourced() { "ok" } else { "warn" } },
                ]
            },
            {
                "kind": "key-value",
                "title": "Aliases",
                "items": items
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "List", "command": "list", "key": "l" },
                    { "label": "Sync", "command": "sync", "key": "y" },
                    { "label": "Status", "command": "status", "key": "s" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "  mac run alias add <name> <command>\n  mac run alias rm <name>\n  mac run alias sync   # ~/.zshrc 에 source 등록 + PATH 추가"
            }
        ],
        "keybindings": [
            { "key": "d", "label": "alias 삭제",
              "command": "rm",
              "args": ["${selected.name}"],
              "confirm": true }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
