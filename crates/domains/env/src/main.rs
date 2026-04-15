use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-env")]
#[command(about = "카드(서비스 연결정보 + 자격증명) 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 카드 목록
    List,
    /// 카드 상세
    Show { name: String },
    /// 카드 추가
    Add {
        name: String,
        #[arg(long)]
        host: String,
        #[arg(long)]
        user: String,
        #[arg(long, default_value_t = 22)]
        port: u16,
        #[arg(long, default_value = "ssh")]
        scheme: String,
        /// 비번. 생략 시 카드만 등록하고 나중에 `env set-password` 로.
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// 카드 삭제 (Keychain 비번도 같이)
    Rm { name: String },
    /// 비번 저장/갱신 (Keychain)
    SetPassword {
        name: String,
        /// 생략 시 stdin 에서 한 줄 읽음
        password: Option<String>,
    },
    /// 비번 조회 (stdout 에 평문. 내부 도메인이 호출)
    GetPassword { name: String },
    /// connections.json + .env 에서 카드로 일괄 이관
    Import,
    /// 카드 ↔ 서버 연결 테스트 (scheme 에 따라)
    Test { name: String },
    /// 상태 요약
    Status,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => cmd_list(),
        Commands::Show { name } => cmd_show(&name),
        Commands::Add { name, host, user, port, scheme, password, description } => {
            cmd_add(&name, &host, &user, port, &scheme, password.as_deref(), description.as_deref())
        }
        Commands::Rm { name } => cmd_rm(&name),
        Commands::SetPassword { name, password } => cmd_set_password(&name, password.as_deref()),
        Commands::GetPassword { name } => cmd_get_password(&name),
        Commands::Import => cmd_import(),
        Commands::Test { name } => cmd_test(&name),
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

// === 데이터 모델 ===

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Card {
    name: String,
    host: String,
    user: String,
    port: u16,
    /// ssh | smb | nfs | http | https | …
    scheme: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    /// "keychain" | "dotenvx:<KEY>" | "none". 기본 "keychain".
    #[serde(default = "default_pw_ref")]
    password_ref: String,
}
fn default_pw_ref() -> String { "keychain".into() }

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

fn cards_dir() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/cards")
}

fn card_path(name: &str) -> PathBuf {
    cards_dir().join(format!("{}.json", name))
}

fn keychain_service(name: &str) -> String {
    format!("mac-app-init:{}", name)
}

fn load_card(name: &str) -> Option<Card> {
    let p = card_path(name);
    if !p.exists() { return None; }
    serde_json::from_str(&fs::read_to_string(&p).ok()?).ok()
}

fn save_card(card: &Card) -> Result<(), String> {
    let dir = cards_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("cards 디렉터리 생성 실패: {}", e))?;
    let json = serde_json::to_string_pretty(card).map_err(|e| format!("{}", e))?;
    fs::write(card_path(&card.name), json).map_err(|e| format!("{}", e))
}

fn list_cards() -> Vec<Card> {
    let dir = cards_dir();
    if !dir.exists() { return Vec::new(); }
    let mut out: Vec<Card> = fs::read_dir(&dir).ok().map(|it| {
        it.filter_map(|e| e.ok())
          .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
          .filter_map(|e| serde_json::from_str::<Card>(&fs::read_to_string(e.path()).ok()?).ok())
          .collect()
    }).unwrap_or_default();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

// === Keychain helpers ===

fn keychain_set(name: &str, password: &str) -> Result<(), String> {
    // -U 플래그로 있으면 업데이트. 기존 삭제 시도는 하지 않음(stderr 소음 방지).
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-s", &keychain_service(name),
            "-a", name,
            "-w", password,
            "-U",
        ])
        .output()
        .map_err(|e| format!("security 실행 실패: {}", e))?;
    if status.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&status.stderr).trim().to_string())
    }
}

fn keychain_get(name: &str) -> Option<String> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-s", &keychain_service(name),
            "-a", name,
            "-w",
        ])
        .output().ok()?;
    if !out.status.success() { return None; }
    let pw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if pw.is_empty() { None } else { Some(pw) }
}

fn keychain_delete(name: &str) -> Result<(), String> {
    let status = Command::new("security")
        .args([
            "delete-generic-password",
            "-s", &keychain_service(name),
            "-a", name,
        ])
        .status()
        .map_err(|e| format!("security 실행 실패: {}", e))?;
    if status.success() { Ok(()) } else { Err("keychain 항목 없음".into()) }
}

// === 커맨드 ===

fn cmd_list() {
    let cards = list_cards();
    if cards.is_empty() {
        println!("카드 없음. `env import` 로 이관하거나 `env add` 로 추가.");
        return;
    }
    println!("{:<14} {:<7} {:<20} {:<22} {}", "NAME", "SCHEME", "USER", "HOST:PORT", "PASSWORD");
    println!("{}", "─".repeat(80));
    for c in cards {
        let hp = format!("{}:{}", c.host, c.port);
        let pw = match c.password_ref.as_str() {
            "keychain" => if keychain_get(&c.name).is_some() { "✓ keychain" } else { "✗ 없음" },
            "none" => "—",
            r if r.starts_with("dotenvx:") => "dotenvx",
            _ => "?",
        };
        println!("{:<14} {:<7} {:<20} {:<22} {}", c.name, c.scheme, c.user, hp, pw);
    }
}

fn cmd_show(name: &str) {
    match load_card(name) {
        Some(c) => {
            // stdout 은 순수 JSON (파서 친화). 부가 정보는 stderr.
            println!("{}", serde_json::to_string_pretty(&c).unwrap());
            if c.password_ref == "keychain" {
                let has = keychain_get(&c.name).is_some();
                eprintln!("비번: {}", if has { "✓ keychain 에 저장됨" } else { "✗ 없음 (env set-password 필요)" });
            }
        }
        None => {
            eprintln!("✗ 카드 '{}' 없음", name);
            std::process::exit(1);
        }
    }
}

fn cmd_add(
    name: &str, host: &str, user: &str, port: u16, scheme: &str,
    password: Option<&str>, description: Option<&str>,
) {
    if load_card(name).is_some() {
        eprintln!("✗ 이미 존재: {}. 변경은 edit 또는 set-password 사용", name);
        std::process::exit(1);
    }
    let card = Card {
        name: name.into(),
        host: host.into(),
        user: user.into(),
        port,
        scheme: scheme.into(),
        description: description.unwrap_or("").into(),
        tags: Vec::new(),
        password_ref: "keychain".into(),
    };
    if let Err(e) = save_card(&card) { eprintln!("✗ {}", e); std::process::exit(1); }
    if let Some(pw) = password {
        if let Err(e) = keychain_set(name, pw) {
            eprintln!("⚠ 카드는 생성됐으나 keychain 저장 실패: {}", e);
        }
    }
    println!("✓ 카드 추가: {}", name);
}

fn cmd_rm(name: &str) {
    let p = card_path(name);
    if !p.exists() {
        eprintln!("✗ 카드 '{}' 없음", name);
        std::process::exit(1);
    }
    if let Err(e) = fs::remove_file(&p) { eprintln!("✗ {}", e); std::process::exit(1); }
    let _ = keychain_delete(name);
    println!("✓ 카드 삭제: {}", name);
}

fn cmd_set_password(name: &str, pw_arg: Option<&str>) {
    if load_card(name).is_none() {
        eprintln!("✗ 카드 '{}' 없음. 먼저 env add", name);
        std::process::exit(1);
    }
    let pw = match pw_arg {
        Some(p) => p.to_string(),
        None => {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).unwrap();
            buf.trim().to_string()
        }
    };
    if pw.is_empty() { eprintln!("✗ 빈 비번"); std::process::exit(1); }
    match keychain_set(name, &pw) {
        Ok(()) => println!("✓ keychain 저장: {}", name),
        Err(e) => { eprintln!("✗ {}", e); std::process::exit(1); }
    }
}

fn cmd_get_password(name: &str) {
    let Some(card) = load_card(name) else {
        eprintln!("✗ 카드 '{}' 없음", name);
        std::process::exit(1);
    };
    let pw = match card.password_ref.as_str() {
        "keychain" => keychain_get(name),
        r if r.starts_with("dotenvx:") => {
            let key = r.trim_start_matches("dotenvx:");
            dotenvx_get(key)
        }
        _ => None,
    };
    match pw {
        Some(p) => print!("{}", p),
        None => { eprintln!("✗ 비번 없음"); std::process::exit(2); }
    }
}

// === import: connections.json + .env → 카드 ===

fn cmd_import() {
    let conn_path = PathBuf::from(home()).join(".mac-app-init/connections.json");
    if !conn_path.exists() {
        eprintln!("✗ {} 없음", conn_path.display());
        std::process::exit(1);
    }
    let content = fs::read_to_string(&conn_path).unwrap_or_default();
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => { eprintln!("✗ JSON 파싱 실패: {}", e); std::process::exit(1); }
    };
    let Some(services) = json.get("services").and_then(|v| v.as_array()) else {
        eprintln!("✗ services[] 없음");
        std::process::exit(1);
    };

    let mut created = 0;
    let mut skipped = 0;
    let mut with_pw = 0;

    for s in services {
        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let host = s.get("host").and_then(|v| v.as_str()).unwrap_or("");
        let user = s.get("user").and_then(|v| v.as_str()).unwrap_or("");
        let port = s.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
        if name.is_empty() { continue; }
        if load_card(name).is_some() {
            skipped += 1;
            continue;
        }
        // port 기반 scheme 추정
        let scheme = match port {
            22 => "ssh",
            139 | 445 => "smb",
            2049 => "nfs",
            80 => "http",
            443 => "https",
            _ => "ssh",
        };
        let card = Card {
            name: name.into(),
            host: host.into(),
            user: user.into(),
            port,
            scheme: scheme.into(),
            description: format!("imported from connections.json"),
            tags: vec!["imported".into()],
            password_ref: "keychain".into(),
        };
        if let Err(e) = save_card(&card) {
            eprintln!("✗ {} 카드 저장 실패: {}", name, e);
            continue;
        }
        created += 1;

        // .env 에서 {NAME}_PASSWORD 읽어 keychain 으로 이관
        let key = format!("{}_PASSWORD", name.to_uppercase().replace('-', "_"));
        if let Some(pw) = dotenvx_get(&key) {
            if let Err(e) = keychain_set(name, &pw) {
                eprintln!("  ⚠ {} keychain 저장 실패: {}", name, e);
            } else {
                with_pw += 1;
            }
        }

        println!("  ✓ {} ({}@{}:{} / {})", name, user, host, port, scheme);
    }
    println!("\nimport: 생성 {}, 이미 있음 {}, 비번 이관 {}", created, skipped, with_pw);
}

fn dotenvx_get(key: &str) -> Option<String> {
    let env_path = PathBuf::from(home()).join(".env");
    if !env_path.exists() { return None; }
    let out = Command::new("dotenvx")
        .args(["get", key, "-f", &env_path.to_string_lossy()])
        .output().ok()?;
    if !out.status.success() { return None; }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() { None } else { Some(v) }
}

// === test: 카드로 가볍게 살아있는지 확인 ===

fn cmd_test(name: &str) {
    let Some(card) = load_card(name) else {
        eprintln!("✗ 카드 '{}' 없음", name);
        std::process::exit(1);
    };
    println!("{} ({}://{}@{}:{}) 테스트 중...", card.name, card.scheme, card.user, card.host, card.port);

    // TCP 연결 가능 여부만 가볍게
    use std::net::ToSocketAddrs;
    use std::time::Duration;
    let addr = format!("{}:{}", card.host, card.port);
    match addr.to_socket_addrs() {
        Ok(mut iter) => {
            if let Some(sock) = iter.next() {
                match std::net::TcpStream::connect_timeout(&sock, Duration::from_secs(3)) {
                    Ok(_) => println!("✓ TCP 연결 성공 ({})", sock),
                    Err(e) => { eprintln!("✗ TCP 실패: {}", e); std::process::exit(2); }
                }
            } else {
                eprintln!("✗ 주소 해석 실패");
                std::process::exit(2);
            }
        }
        Err(e) => { eprintln!("✗ 주소 해석 실패: {}", e); std::process::exit(2); }
    }
}

// === status ===

fn cmd_status() {
    let cards = list_cards();
    println!("=== Env Status ===\n");
    println!("카드 {}개", cards.len());
    let kc = cards.iter().filter(|c| c.password_ref == "keychain" && keychain_get(&c.name).is_some()).count();
    let dx = cards.iter().filter(|c| c.password_ref.starts_with("dotenvx:")).count();
    let none = cards.iter().filter(|c| c.password_ref == "none" || (c.password_ref == "keychain" && keychain_get(&c.name).is_none())).count();
    println!("  • keychain 비번: {}", kc);
    println!("  • dotenvx 비번: {}", dx);
    println!("  • 비번 없음:     {}", none);
    let conn_path = PathBuf::from(home()).join(".mac-app-init/connections.json");
    if conn_path.exists() {
        println!("\n⚠ legacy {} 존재 — `env import` 권장", conn_path.display());
    }
}

fn print_tui_spec() {
    let cards = list_cards();
    let items: Vec<serde_json::Value> = cards.iter().map(|c| {
        let has_pw = c.password_ref == "keychain" && keychain_get(&c.name).is_some();
        serde_json::json!({
            "key": c.name,
            "value": format!("{}://{}@{}:{}", c.scheme, c.user, c.host, c.port),
            "status": if has_pw { "ok" } else { "warn" }
        })
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label": "Env", "icon": "🔑" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Cards",
                "items": items
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "List", "command": "list", "key": "l" },
                    { "label": "Import legacy", "command": "import", "key": "i" },
                    { "label": "Status", "command": "status", "key": "s" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "카드 = 서비스 연결정보 + 자격증명. 비번은 macOS Keychain 에 안전 저장."
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
