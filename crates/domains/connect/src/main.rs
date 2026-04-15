use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-connect")]
#[command(about = "외부 서비스 연결 관리 (.env + dotenvx 암호화)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 새 연결 추가
    Add {
        /// 서비스 이름 (proxmox, synology, etc)
        name: String,
    },
    /// 연결 삭제
    Remove {
        /// 서비스 이름
        name: String,
    },
    /// 등록된 연결 목록
    List,
    /// 전체 연결 상태 (ping/ssh 테스트)
    Status,
    /// 특정 연결 테스트
    Test {
        /// 서비스 이름
        name: String,
    },
    /// .env 다시 암호화
    Encrypt,
    /// .env 에서 연결 정보 스캔 → connections.json 병합
    Import,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Connection {
    name: String,
    host: String,
    user: String,
    port: u16,
    #[serde(default)]
    extra: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Connections {
    services: Vec<Connection>,
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

fn env_path() -> PathBuf {
    PathBuf::from(home()).join(".env")
}

fn connections_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/connections.json")
}

fn load_connections() -> Connections {
    let path = connections_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Connections::default()
    }
}

fn save_connections(conns: &Connections) {
    let path = connections_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    let json = serde_json::to_string_pretty(conns).unwrap();
    fs::write(&path, json).expect("connections.json 저장 실패");
}

fn prompt(label: &str, default: &str) -> String {
    if default.is_empty() {
        print!("  {}: ", label);
    } else {
        print!("  {} [{}]: ", label, default);
    }
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn env_key(service: &str, field: &str) -> String {
    format!("{}_{}", service.to_uppercase().replace('-', "_"), field.to_uppercase())
}

fn append_env(key: &str, value: &str) {
    let path = env_path();
    let content = fs::read_to_string(&path).unwrap_or_default();

    // Replace if exists, append if not
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let found = lines.iter_mut().any(|l| {
        if l.starts_with(&format!("{}=", key)) || l.starts_with(&format!("{}=encrypted:", key)) {
            *l = format!("{}={}", key, value);
            true
        } else {
            false
        }
    });
    if !found {
        lines.push(format!("{}={}", key, value));
    }

    fs::write(&path, lines.join("\n") + "\n").ok();
}

fn remove_env(prefix: &str) {
    let path = env_path();
    let content = fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<&str> = content
        .lines()
        .filter(|l| !l.starts_with(&format!("{}=", prefix)) && !l.starts_with(&format!("{}=encrypted:", prefix)))
        .collect();
    fs::write(&path, lines.join("\n") + "\n").ok();
}

fn dotenvx_encrypt() {
    let path = env_path();
    let output = Command::new("dotenvx")
        .args(["encrypt", "-f", &path.to_string_lossy()])
        .output();
    match output {
        Ok(o) if o.status.success() => println!("  ✓ .env 암호화 완료"),
        Ok(o) => println!("  ⚠ 암호화 실패: {}", String::from_utf8_lossy(&o.stderr).trim()),
        Err(_) => println!("  ⚠ dotenvx 미설치 (brew install dotenvx/brew/dotenvx)"),
    }
}

fn test_connection(conn: &Connection) -> bool {
    // Try SSH first
    let output = Command::new("ssh")
        .args([
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=3",
            "-p", &conn.port.to_string(),
            &format!("{}@{}", conn.user, conn.host),
            "echo ok",
        ])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => {
            // Fallback: ping
            let ping = Command::new("ping")
                .args(["-c", "1", "-W", "3", &conn.host])
                .output();
            ping.map(|o| o.status.success()).unwrap_or(false)
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Add { name } => cmd_add(&name),
        Commands::Remove { name } => cmd_remove(&name),
        Commands::List => cmd_list(),
        Commands::Status => cmd_status(),
        Commands::Test { name } => cmd_test(&name),
        Commands::Encrypt => dotenvx_encrypt(),
        Commands::Import => cmd_import(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let conns = load_connections();
    let rows: Vec<serde_json::Value> = conns.services.iter().map(|c| {
        serde_json::json!([
            c.name,
            c.host,
            c.user,
            c.port.to_string(),
        ])
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label": "Connect", "icon": "🔌" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    {
                        "key": "등록된 연결",
                        "value": format!("{} 개", conns.services.len()),
                        "status": if conns.services.is_empty() { "warn" } else { "ok" }
                    }
                ]
            },
            {
                "kind": "table",
                "title": "연결 목록",
                "headers": ["NAME", "HOST", "USER", "PORT"],
                "rows": rows
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status (ping/ssh 테스트)", "command": "status", "key": "s" },
                    { "label": "Import (.env → connections.json)", "command": "import", "key": "i" },
                    { "label": "Encrypt (.env 재암호화)", "command": "encrypt", "key": "e" }
                ]
            },
            {
                "kind": "text",
                "title": "등록 / 삭제 / 수정 — 터미널에서 (stdin 입력 필요)",
                "content": "  추가:  mac run connect add <name>\n  삭제:  mac run connect remove <name>\n  테스트: mac run connect test <name>\n\n  TUI는 입력 폼을 아직 지원하지 않아 대화형 명령은 터미널에서 실행해야 합니다."
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}

fn cmd_add(name: &str) {
    let mut conns = load_connections();

    if conns.services.iter().any(|c| c.name == name) {
        println!("'{}' 이미 등록되어 있습니다.", name);
        return;
    }

    println!("=== {} 연결 추가 ===\n", name);
    let host = prompt("Host (IP 또는 도메인)", "");
    let user = prompt("User", "root");
    let port: u16 = prompt("Port", "22").parse().unwrap_or(22);

    // Extra fields
    let mut extra = HashMap::new();
    println!("\n  추가 설정 (빈 줄로 종료):");
    loop {
        let key = prompt("  키 (예: PASSWORD)", "");
        if key.is_empty() {
            break;
        }
        let value = prompt(&format!("  {}", key), "");
        if !value.is_empty() {
            extra.insert(key, value);
        }
    }

    let conn = Connection {
        name: name.to_string(),
        host: host.clone(),
        user: user.clone(),
        port,
        extra: extra.clone(),
    };

    // Save to connections.json
    conns.services.push(conn);
    save_connections(&conns);

    // Save to .env
    append_env(&env_key(name, "HOST"), &host);
    append_env(&env_key(name, "USER"), &user);
    append_env(&env_key(name, "PORT"), &port.to_string());
    for (k, v) in &extra {
        append_env(&env_key(name, k), v);
    }

    // Encrypt
    dotenvx_encrypt();

    println!("\n✓ {} 연결 추가 완료", name);
    println!("  {}@{}:{}", user, host, port);
}

fn cmd_remove(name: &str) {
    let mut conns = load_connections();
    let before = conns.services.len();
    conns.services.retain(|c| c.name != name);

    if conns.services.len() == before {
        println!("'{}' 등록되어 있지 않습니다.", name);
        return;
    }

    save_connections(&conns);

    // Remove from .env
    let prefix = name.to_uppercase().replace('-', "_");
    remove_env(&format!("{}_HOST", prefix));
    remove_env(&format!("{}_USER", prefix));
    remove_env(&format!("{}_PORT", prefix));
    remove_env(&format!("{}_PASSWORD", prefix));

    dotenvx_encrypt();
    println!("✓ {} 연결 삭제 완료", name);
}

fn cmd_list() {
    let conns = load_connections();
    if conns.services.is_empty() {
        println!("등록된 연결이 없습니다.");
        println!("  mac run connect add proxmox");
        return;
    }

    println!("{:<15} {:<20} {:<10} {}", "NAME", "HOST", "USER", "PORT");
    println!("{}", "─".repeat(55));
    for c in &conns.services {
        println!("{:<15} {:<20} {:<10} {}", c.name, c.host, c.user, c.port);
    }
}

fn cmd_status() {
    let conns = load_connections();
    if conns.services.is_empty() {
        println!("등록된 연결이 없습니다.");
        return;
    }

    println!("=== 연결 상태 ===\n");
    for c in &conns.services {
        let ok = test_connection(c);
        let icon = if ok { "✓" } else { "✗" };
        println!(
            "  {} {:<15} {}@{}:{}",
            icon, c.name, c.user, c.host, c.port
        );
    }
}

fn dotenvx_get(key: &str) -> Option<String> {
    let path = env_path();
    let out = Command::new("dotenvx")
        .args(["get", key, "-f", &path.to_string_lossy()])
        .output()
        .ok()?;
    if !out.status.success() { return None; }
    let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if val.is_empty() || val == "undefined" { None } else { Some(val) }
}

/// .env 에서 `<NAME>_{HOST|USER|PORT|PASSWORD|...}` 패턴으로 서비스 이름 수집
fn scan_env_services() -> Vec<String> {
    // SSH/네트워크 연결 식별자만 — API 키/토큰은 제외
    const SUFFIXES: &[&str] = &["_HOST", "_PORT"];
    let content = fs::read_to_string(env_path()).unwrap_or_default();
    let mut names: Vec<String> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((key, _)) = line.split_once('=') else { continue; };
        for suf in SUFFIXES {
            if let Some(prefix) = key.strip_suffix(suf) {
                let name = prefix.to_lowercase().replace('_', "-");
                if !name.is_empty() && !names.contains(&name) {
                    names.push(name);
                }
                break;
            }
        }
    }
    names
}

fn cmd_import() {
    let services = scan_env_services();
    if services.is_empty() {
        println!(".env 에서 *_HOST 키를 찾지 못했습니다.");
        return;
    }

    let mut conns = load_connections();
    let mut added = 0;
    let mut skipped = 0;

    for name in &services {
        if conns.services.iter().any(|c| &c.name == name) {
            skipped += 1;
            continue;
        }
        let prefix = name.to_uppercase().replace('-', "_");
        let host = dotenvx_get(&format!("{}_HOST", prefix)).unwrap_or_else(|| "—".into());
        let user = dotenvx_get(&format!("{}_USER", prefix)).unwrap_or_else(|| "—".into());
        let port: u16 = dotenvx_get(&format!("{}_PORT", prefix))
            .and_then(|s| s.parse().ok())
            .unwrap_or(22);

        conns.services.push(Connection {
            name: name.clone(),
            host: host.clone(),
            user: user.clone(),
            port,
            extra: HashMap::new(),
        });
        println!("  ✓ {} ({}@{}:{})", name, user, host, port);
        added += 1;
    }

    if added > 0 { save_connections(&conns); }
    println!("\nimport 완료: 추가 {}, 기존 {}", added, skipped);
}

fn cmd_test(name: &str) {
    let conns = load_connections();
    match conns.services.iter().find(|c| c.name == name) {
        Some(c) => {
            println!("Testing {}@{}:{}...", c.user, c.host, c.port);
            if test_connection(c) {
                println!("  ✓ 연결 성공");
            } else {
                println!("  ✗ 연결 실패");
            }
        }
        None => println!("'{}' 등록되어 있지 않습니다.", name),
    }
}
