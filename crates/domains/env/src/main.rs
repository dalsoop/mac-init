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
    /// 마운트 옵션 변경 (key=readonly|noappledouble|soft|nobrowse, value=true|false)
    SetOption {
        name: String,
        key: String,
        value: String,
    },
    /// connections.json + .env 에서 카드로 일괄 이관
    Import,
    /// 카드 ↔ 서버 연결 테스트 (scheme 에 따라)
    Test { name: String },
    /// 상태 요약
    Status,
    /// 모든 카드 파일 권한을 0600 (디렉터리 0700) 으로 보정
    FixPerms,
    /// 기존 keychain 비번을 dotenvx 로 일괄 이관 + Keychain 항목 삭제
    MigrateFromKeychain,
    /// import 후 남은 connections.json / connections.json.legacy-backup 정리
    Cleanup {
        /// 실제 삭제 (--apply 없으면 dry-run)
        #[arg(long)]
        apply: bool,
    },
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
        Commands::SetOption { name, key, value } => cmd_set_option(&name, &key, &value),
        Commands::Import => cmd_import(),
        Commands::Test { name } => cmd_test(&name),
        Commands::Status => cmd_status(),
        Commands::FixPerms => cmd_fix_perms(),
        Commands::MigrateFromKeychain => cmd_migrate_from_keychain(),
        Commands::Cleanup { apply } => cmd_cleanup(apply),
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
    /// "dotenvx:<KEY>" | "none". 기본은 "dotenvx:{NAME}_PASSWORD".
    /// (codesign 없는 빌드에서 Keychain ACL 이 매번 깨지는 문제로 dotenvx 단일화)
    #[serde(default = "default_pw_ref")]
    password_ref: String,
    /// 마운트/접속 옵션. SMB/NFS 마운트 시 mount 도메인이 참조.
    #[serde(default)]
    mount_options: MountOptions,
}
fn default_pw_ref() -> String { "dotenvx:auto".into() }

/// 카드 이름 → dotenvx 키. password_ref 가 "dotenvx:auto" 면 자동 매핑,
/// "dotenvx:<EXPLICIT_KEY>" 면 그 키 그대로.
fn dotenvx_key_for(card: &Card) -> Option<String> {
    let r = &card.password_ref;
    if r == "dotenvx:auto" {
        Some(format!("{}_PASSWORD", card.name.to_uppercase().replace('-', "_")))
    } else if let Some(k) = r.strip_prefix("dotenvx:") {
        Some(k.to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MountOptions {
    /// 읽기 전용 마운트 (mount_smbfs -o rdonly / NetFS MNT_RDONLY)
    #[serde(default)]
    readonly: bool,
    /// .DS_Store / ._* 생성 억제 (NAS 노이즈 방지)
    #[serde(default = "default_true_opt")]
    noappledouble: bool,
    /// 서버 무응답 시 hang 대신 EIO (mount_smbfs -o soft)
    #[serde(default = "default_true_opt")]
    soft: bool,
    /// Finder 사이드바에 노출 안 함 (mount_smbfs -o nobrowse)
    #[serde(default = "default_true_opt")]
    nobrowse: bool,
}
fn default_true_opt() -> bool { true }

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            readonly: false,
            noappledouble: true,
            soft: true,
            nobrowse: true,
        }
    }
}

impl MountOptions {
    /// 스킴별 권장 기본값. 추후 NFS 등 분기 가능.
    fn default_for_scheme(_scheme: &str) -> Self { Self::default() }
}

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
    // 디렉터리는 0700 (다른 유저가 cards 목록 자체를 못 보게)
    let _ = set_mode(&dir, 0o700);
    let json = serde_json::to_string_pretty(card).map_err(|e| format!("{}", e))?;
    let path = card_path(&card.name);
    fs::write(&path, json).map_err(|e| format!("{}", e))?;
    // 파일은 0600 (host/user 노출 방지)
    let _ = set_mode(&path, 0o600);
    Ok(())
}

#[cfg(unix)]
fn set_mode(p: &std::path::Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(p)?.permissions();
    perm.set_mode(mode);
    fs::set_permissions(p, perm)
}
#[cfg(not(unix))]
fn set_mode(_p: &std::path::Path, _mode: u32) -> std::io::Result<()> { Ok(()) }

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

#[allow(dead_code)]
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
    let out = Command::new("security")
        .args([
            "delete-generic-password",
            "-s", &keychain_service(name),
            "-a", name,
        ])
        .output()
        .map_err(|e| format!("security 실행 실패: {}", e))?;
    if out.status.success() { Ok(()) } else { Err("keychain 항목 없음".into()) }
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
    for c in &cards {
        let hp = format!("{}:{}", c.host, c.port);
        let pw = match dotenvx_key_for(c) {
            Some(k) => if dotenvx_get(&k).is_some() { "✓ dotenvx" } else { "✗ 없음" },
            None => "—",
        };
        println!("{:<14} {:<7} {:<20} {:<22} {}", c.name, c.scheme, c.user, hp, pw);
    }
}

fn cmd_show(name: &str) {
    match load_card(name) {
        Some(c) => {
            // stdout 은 순수 JSON (파서 친화). 부가 정보는 stderr.
            println!("{}", serde_json::to_string_pretty(&c).unwrap());
            if let Some(k) = dotenvx_key_for(&c) {
                let has = dotenvx_get(&k).is_some();
                eprintln!("비번: {} (key={})", if has { "✓ dotenvx" } else { "✗ 없음 (env set-password 필요)" }, k);
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
        password_ref: "dotenvx:auto".into(),
        mount_options: MountOptions::default_for_scheme(scheme),
    };
    if let Err(e) = save_card(&card) { eprintln!("✗ {}", e); std::process::exit(1); }
    if let Some(pw) = password {
        if let Some(k) = dotenvx_key_for(&card) {
            if let Err(e) = dotenvx_set(&k, pw) {
                eprintln!("⚠ 카드는 생성됐으나 dotenvx 저장 실패: {}", e);
            }
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
    let card = load_card(name);
    if let Err(e) = fs::remove_file(&p) { eprintln!("✗ {}", e); std::process::exit(1); }
    if let Some(c) = card.as_ref() {
        if let Some(k) = dotenvx_key_for(c) {
            let _ = dotenvx_unset(&k);
        }
    }
    // legacy keychain 도 함께 정리
    let _ = keychain_delete(name);
    println!("✓ 카드 삭제: {}", name);
}

fn cmd_set_password(name: &str, pw_arg: Option<&str>) {
    let Some(card) = load_card(name) else {
        eprintln!("✗ 카드 '{}' 없음. 먼저 env add", name);
        std::process::exit(1);
    };
    let pw = match pw_arg {
        Some(p) => p.to_string(),
        None => {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).unwrap();
            buf.trim().to_string()
        }
    };
    if pw.is_empty() { eprintln!("✗ 빈 비번"); std::process::exit(1); }
    let Some(key) = dotenvx_key_for(&card) else {
        eprintln!("✗ 카드의 password_ref 가 dotenvx 가 아님");
        std::process::exit(1);
    };
    match dotenvx_set(&key, &pw) {
        Ok(()) => println!("✓ dotenvx 저장: {}={}", key, "***"),
        Err(e) => { eprintln!("✗ {}", e); std::process::exit(1); }
    }
}

fn cmd_get_password(name: &str) {
    let Some(card) = load_card(name) else {
        eprintln!("✗ 카드 '{}' 없음", name);
        std::process::exit(1);
    };
    let pw = dotenvx_key_for(&card).and_then(|k| dotenvx_get(&k));
    match pw {
        Some(p) => print!("{}", p),
        None => { eprintln!("✗ 비번 없음"); std::process::exit(2); }
    }
}

// === import: connections.json + .env → 카드 ===

fn cmd_set_option(name: &str, key: &str, value: &str) {
    let Some(mut card) = load_card(name) else {
        eprintln!("✗ 카드 '{}' 없음", name);
        std::process::exit(1);
    };
    let v: bool = match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => true,
        "false" | "0" | "off" | "no" => false,
        _ => { eprintln!("✗ value 는 true|false"); std::process::exit(1); }
    };
    let mut opts = card.mount_options.clone();
    match key {
        "readonly" => opts.readonly = v,
        "noappledouble" => opts.noappledouble = v,
        "soft" => opts.soft = v,
        "nobrowse" => opts.nobrowse = v,
        _ => {
            eprintln!("✗ key 는 readonly|noappledouble|soft|nobrowse");
            std::process::exit(1);
        }
    }
    card.mount_options = opts;
    if let Err(e) = save_card(&card) { eprintln!("✗ {}", e); std::process::exit(1); }
    println!("✓ {} {}={}", name, key, v);

    // 현재 마운트된 share 가 있으면 자동 재마운트 (옵션 즉시 적용).
    let remounted = remount_active_shares(name);
    if remounted > 0 {
        println!("  ↻ 활성 마운트 {}개 재마운트됨 (옵션 적용)", remounted);
    }
}

/// `mount` 출력에서 "//user@host/<share> on <mp>" 패턴 중 우리가 만든
/// ~/NAS/<card>/<share> 위치인 것을 찾아 unmount → mac-domain-mount mount 재호출.
fn remount_active_shares(card_name: &str) -> usize {
    let mount_bin = mount_binary();
    let nas_prefix = format!("{}/NAS/{}/", home(), card_name);
    let out = match Command::new("mount").output() {
        Ok(o) => o,
        Err(_) => return 0,
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut count = 0;
    for line in stdout.lines() {
        // 예: //ai@192.168.2.15/works on /Users/jeonghan/NAS/synology/works (smbfs, ...)
        let Some(on_idx) = line.find(" on ") else { continue; };
        let mp = &line[on_idx + 4..];
        let Some(paren) = mp.find(" (") else { continue; };
        let mp_path = &mp[..paren];
        if !mp_path.starts_with(&nas_prefix) { continue; }
        let Some(share) = mp_path.strip_prefix(&nas_prefix) else { continue; };

        // unmount 후 재마운트
        let _ = Command::new(&mount_bin).args(["unmount", mp_path]).status();
        let _ = Command::new(&mount_bin).args(["mount", card_name, share]).status();
        count += 1;
    }
    count
}

fn mount_binary() -> PathBuf {
    let candidates = [
        PathBuf::from(home()).join(".mac-app-init/domains/mac-domain-mount"),
        PathBuf::from("./target/debug/mac-domain-mount"),
        PathBuf::from("./target/release/mac-domain-mount"),
    ];
    for c in &candidates {
        if c.exists() { return c.clone(); }
    }
    PathBuf::from("mac-domain-mount")
}

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
        // scheme 추정: port 우선, 비정형 포트면 실제 TCP 프로브.
        let scheme = guess_scheme(host, port);
        // SMB 추정인데 port 가 445/139 가 아니면 DSM UI 포트 등이 연결 저장된 것.
        // 카드에는 실제 서비스 포트로 보정 저장.
        let effective_port = match (scheme, port) {
            ("smb", p) if p != 445 && p != 139 => 445,
            ("nfs", p) if p != 2049 => 2049,
            (_, p) => p,
        };
        if effective_port != port {
            eprintln!("  ↻ {} port 보정: {} → {} ({})", name, port, effective_port, scheme);
        }
        let card = Card {
            name: name.into(),
            host: host.into(),
            user: user.into(),
            port: effective_port,
            scheme: scheme.into(),
            description: format!("imported from connections.json"),
            tags: vec!["imported".into()],
            password_ref: "dotenvx:auto".into(),
            mount_options: MountOptions::default_for_scheme(scheme),
        };
        if let Err(e) = save_card(&card) {
            eprintln!("✗ {} 카드 저장 실패: {}", name, e);
            continue;
        }
        created += 1;

        // .env 에 {NAME}_PASSWORD 가 이미 있는지 확인 (있으면 그대로 사용 — 추가 작업 없음)
        let key = format!("{}_PASSWORD", name.to_uppercase().replace('-', "_"));
        if dotenvx_get(&key).is_some() {
            with_pw += 1;
        }

        println!("  ✓ {} ({}@{}:{} / {})", name, user, host, effective_port, scheme);
    }
    println!("\nimport: 생성 {}, 이미 있음 {}, dotenvx 비번 매칭 {}", created, skipped, with_pw);
}

/// 포트 기반 1차 추정. 비표준 포트면 host 에 SMB(445)/SSH(22) TCP 프로브.
fn guess_scheme(host: &str, port: u16) -> &'static str {
    match port {
        22 => return "ssh",
        139 | 445 => return "smb",
        2049 => return "nfs",
        80 => return "http",
        443 => return "https",
        _ => {}
    }
    // 비정형 포트: 445 → 22 순으로 프로브
    for (p, s) in [(445u16, "smb"), (22u16, "ssh")] {
        if probe_tcp(host, p) { return s; }
    }
    "ssh"
}

fn probe_tcp(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    use std::time::Duration;
    let addr = format!("{}:{}", host, port);
    if let Ok(mut it) = addr.to_socket_addrs() {
        if let Some(sock) = it.next() {
            return std::net::TcpStream::connect_timeout(&sock, Duration::from_millis(500)).is_ok();
        }
    }
    false
}

fn env_file() -> PathBuf {
    PathBuf::from(home()).join(".env")
}

fn dotenvx_get(key: &str) -> Option<String> {
    let p = env_file();
    if !p.exists() { return None; }
    let out = Command::new("dotenvx")
        .args(["get", key, "-f", &p.to_string_lossy()])
        .output().ok()?;
    if !out.status.success() { return None; }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() { None } else { Some(v) }
}

fn dotenvx_set(key: &str, value: &str) -> Result<(), String> {
    let p = env_file();
    // ~/.env 가 없으면 빈 파일 생성
    if !p.exists() {
        fs::write(&p, "").map_err(|e| format!("~/.env 생성 실패: {}", e))?;
        let _ = set_mode(&p, 0o600);
    }
    let out = Command::new("dotenvx")
        .args(["set", key, value, "-f", &p.to_string_lossy(), "--encrypt"])
        .output()
        .map_err(|e| format!("dotenvx 실행 실패: {} (brew install dotenvx/brew/dotenvx)", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let _ = set_mode(&p, 0o600);
    Ok(())
}

fn dotenvx_unset(key: &str) -> Result<(), String> {
    let p = env_file();
    if !p.exists() { return Ok(()); }
    let out = Command::new("dotenvx")
        .args(["set", key, "", "-f", &p.to_string_lossy()])
        .output()
        .map_err(|e| format!("dotenvx 실행 실패: {}", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
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
    let mut with = 0usize; let mut without = 0usize;
    for c in &cards {
        match dotenvx_key_for(c) {
            Some(k) if dotenvx_get(&k).is_some() => with += 1,
            _ => without += 1,
        }
    }
    println!("  • dotenvx 비번 있음: {}", with);
    println!("  • 비번 없음:         {}", without);

    // 권한 점검
    let perms = audit_permissions();
    if !perms.is_empty() {
        println!("\n⚠ 권한 부적합 ({}개 — `env fix-perms` 로 0600 적용):", perms.len());
        for p in perms.iter().take(5) { println!("  • {}", p); }
    } else if !cards.is_empty() {
        println!("  • 파일 권한:    ✓ 0600");
    }

    let conn_path = PathBuf::from(home()).join(".mac-app-init/connections.json");
    if conn_path.exists() {
        println!("\n⚠ legacy {} 존재 — `env import` 권장", conn_path.display());
    }
}

fn audit_permissions() -> Vec<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = cards_dir();
        if !dir.exists() { return Vec::new(); }
        let mut bad = Vec::new();
        if let Ok(it) = fs::read_dir(&dir) {
            for e in it.filter_map(|x| x.ok()) {
                let path = e.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
                if let Ok(meta) = fs::metadata(&path) {
                    let mode = meta.permissions().mode() & 0o777;
                    if mode != 0o600 {
                        bad.push(format!("{} (현재 {:o})", path.display(), mode));
                    }
                }
            }
        }
        bad
    }
    #[cfg(not(unix))]
    { Vec::new() }
}

fn cmd_migrate_from_keychain() {
    let cards = list_cards();
    let mut moved = 0; let mut skipped = 0; let mut failed = 0;
    for mut c in cards {
        // legacy: password_ref == "keychain" 이거나, dotenvx 키에 비번이 없는데
        // keychain 에는 있는 경우.
        let kc_pw = keychain_get(&c.name);
        let dx_key = format!("{}_PASSWORD", c.name.to_uppercase().replace('-', "_"));
        let dx_has = dotenvx_get(&dx_key).is_some();

        let need_move = c.password_ref == "keychain" || (kc_pw.is_some() && !dx_has);
        if !need_move {
            // password_ref 만 신식으로 정리
            if c.password_ref == "keychain" || c.password_ref.is_empty() {
                c.password_ref = "dotenvx:auto".into();
                let _ = save_card(&c);
            }
            skipped += 1;
            continue;
        }

        // 1) keychain → dotenvx 복사 (없으면 skip)
        if let Some(pw) = kc_pw {
            if let Err(e) = dotenvx_set(&dx_key, &pw) {
                eprintln!("✗ {} dotenvx 저장 실패: {}", c.name, e);
                failed += 1;
                continue;
            }
        } else {
            eprintln!("⚠ {} keychain 비번 없음 (카드만 갱신)", c.name);
        }

        // 2) 카드 password_ref 갱신
        c.password_ref = "dotenvx:auto".into();
        if let Err(e) = save_card(&c) {
            eprintln!("✗ {} 카드 저장 실패: {}", c.name, e);
            failed += 1;
            continue;
        }

        // 3) keychain 항목 삭제
        let _ = keychain_delete(&c.name);

        println!("  ✓ {} → dotenvx ({})", c.name, dx_key);
        moved += 1;
    }
    println!("\nmigrate: 이관 {}, skip {}, 실패 {}", moved, skipped, failed);
}

fn cmd_cleanup(apply: bool) {
    let dir = PathBuf::from(home()).join(".mac-app-init");
    let candidates = [
        dir.join("connections.json"),
        dir.join("connections.json.legacy-backup"),
        dir.join("connections.json.bak"),
    ];
    let mut found = 0;
    for p in &candidates {
        if !p.exists() { continue; }
        found += 1;
        if apply {
            match fs::remove_file(p) {
                Ok(()) => println!("  ✓ 삭제: {}", p.display()),
                Err(e) => eprintln!("  ✗ {}: {}", p.display(), e),
            }
        } else {
            println!("  • [dry-run] 삭제 대상: {}", p.display());
        }
    }
    if found == 0 {
        println!("정리할 legacy 파일 없음.");
    } else if !apply {
        println!("\n실제 삭제하려면 `env cleanup --apply`");
    }
}

fn cmd_fix_perms() {
    let dir = cards_dir();
    let _ = set_mode(&dir, 0o700);
    let mut fixed = 0;
    if let Ok(it) = fs::read_dir(&dir) {
        for e in it.filter_map(|x| x.ok()) {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
            if set_mode(&path, 0o600).is_ok() { fixed += 1; }
        }
    }
    println!("✓ {} 개 카드 파일 권한 0600 적용 (디렉터리 0700)", fixed);
}

fn print_tui_spec() {
    let cards = list_cards();
    let items: Vec<serde_json::Value> = cards.iter().map(|c| {
        let has_pw = dotenvx_key_for(c).is_some_and(|k| dotenvx_get(&k).is_some());
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
