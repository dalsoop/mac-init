use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-wireguard")]
#[command(about = "WireGuard VPN 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 확인
    Status,
    /// WireGuard 설치 (wg CLI + GUI app)
    Install,
    /// 연결 설정 목록
    List,
    /// 연결 시작
    Up { name: String },
    /// 연결 정지
    Down { name: String },
    /// GUI 앱 열기
    Open,
    /// 설정 추가 (conf 파일 경로)
    Add {
        name: String,
        /// .conf 파일 경로
        conf: String,
    },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn home() -> String { std::env::var("HOME").unwrap_or_default() }

fn cmd_ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).output().map(|o| o.status.success()).unwrap_or(false)
}

fn cmd_stdout(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn config_dir() -> PathBuf {
    // Try multiple locations
    let candidates = [
        "/opt/homebrew/etc/wireguard",
        "/usr/local/etc/wireguard",
    ];
    for p in &candidates {
        if PathBuf::from(p).is_dir() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from("/opt/homebrew/etc/wireguard")
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Install => cmd_install(),
        Commands::List => cmd_list(),
        Commands::Up { name } => cmd_up(&name),
        Commands::Down { name } => cmd_down(&name),
        Commands::Open => cmd_open(),
        Commands::Add { name, conf } => cmd_add(&name, &conf),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let wg_cli = cmd_ok("which", &["wg"]);
    let gui = std::path::Path::new("/Applications/WireGuard.app").exists();
    let cfg_dir = config_dir();

    let mut rows: Vec<serde_json::Value> = Vec::new();
    let active = if wg_cli { cmd_stdout("wg", &["show", "interfaces"]) } else { String::new() };
    let active_list: Vec<&str> = active.split_whitespace().collect();

    if let Ok(entries) = fs::read_dir(&cfg_dir) {
        for e in entries.flatten() {
            if e.path().extension().map(|x| x == "conf").unwrap_or(false) {
                let name = e.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let running = active_list.contains(&name.as_str());
                rows.push(serde_json::json!([
                    if running { "●" } else { " " }.to_string(),
                    name,
                    e.path().display().to_string(),
                ]));
            }
        }
    }

    let spec = serde_json::json!({
        "tab": { "label": "WireGuard", "icon": "🔒" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    {
                        "key": "wg CLI",
                        "value": if wg_cli { "✓ 설치됨" } else { "✗ 미설치" },
                        "status": if wg_cli { "ok" } else { "error" }
                    },
                    {
                        "key": "WireGuard.app",
                        "value": if gui { "✓ 설치됨" } else { "✗ 미설치" },
                        "status": if gui { "ok" } else { "warn" }
                    },
                    {
                        "key": "설정 디렉토리",
                        "value": cfg_dir.display().to_string(),
                        "status": "ok"
                    }
                ]
            },
            {
                "kind": "table",
                "title": "Configs",
                "headers": ["", "NAME", "PATH"],
                "rows": rows
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status (상태)", "command": "status", "key": "s" },
                    { "label": "Install (wg + GUI 설치)", "command": "install", "key": "i" },
                    { "label": "List (설정 목록)", "command": "list", "key": "l" },
                    { "label": "Open (GUI 열기)", "command": "open", "key": "o" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}

fn cmd_status() {
    println!("=== WireGuard 상태 ===\n");

    // wg CLI
    let wg_cli = cmd_ok("which", &["wg"]);
    if wg_cli {
        let ver = cmd_stdout("wg", &["--version"]);
        println!("[wg CLI] ✓ {}", ver);
    } else {
        println!("[wg CLI] ✗ 미설치");
        println!("  → mac run wireguard install");
    }

    // GUI app
    let gui = std::path::Path::new("/Applications/WireGuard.app").exists();
    println!("[WireGuard.app] {}", if gui { "✓ 설치됨" } else { "✗ 미설치" });

    // Configs
    let cfg_dir = config_dir();
    println!("\n[설정 디렉토리] {}", cfg_dir.display());
    if cfg_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&cfg_dir) {
            let confs: Vec<String> = entries.flatten()
                .filter(|e| e.path().extension().map(|x| x == "conf").unwrap_or(false))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            println!("[설정 파일] {} 개", confs.len());
            for c in &confs {
                println!("  {}", c);
            }
        }
    } else {
        println!("  (없음)");
    }

    // Active interfaces
    if wg_cli {
        let active = cmd_stdout("wg", &["show"]);
        println!("\n[활성 연결]");
        if active.is_empty() {
            println!("  없음");
        } else {
            for line in active.lines().take(20) {
                println!("  {}", line);
            }
        }
    }
}

fn cmd_install() {
    let mut installed_any = false;

    if !cmd_ok("which", &["wg"]) {
        println!("[wg CLI 설치 중...]");
        let ok = Command::new("brew").args(["install", "wireguard-tools"]).status()
            .map(|s| s.success()).unwrap_or(false);
        println!("  {}", if ok { "✓ wireguard-tools 설치 완료" } else { "✗ 설치 실패" });
        installed_any = true;
    } else {
        println!("[wg CLI] ✓ 이미 설치됨");
    }

    if !std::path::Path::new("/Applications/WireGuard.app").exists() {
        println!("[WireGuard.app 설치 중...]");
        let ok = Command::new("brew").args(["install", "--cask", "wireguard"]).status()
            .map(|s| s.success()).unwrap_or(false);
        println!("  {}", if ok { "✓ WireGuard.app 설치 완료" } else { "✗ 설치 실패 (Mac App Store에서 설치 권장)" });
        installed_any = true;
    } else {
        println!("[WireGuard.app] ✓ 이미 설치됨");
    }

    // Ensure config dir
    let cfg_dir = config_dir();
    fs::create_dir_all(&cfg_dir).ok();

    if installed_any {
        println!("\n=== 설치 완료 ===");
    }
    println!("  설정 추가: mac run wireguard add <name> <path/to/conf>");
    println!("  연결: mac run wireguard up <name>");
}

fn cmd_list() {
    let cfg_dir = config_dir();
    if !cfg_dir.is_dir() {
        println!("설정 디렉토리가 없습니다: {}", cfg_dir.display());
        return;
    }

    println!("=== WireGuard 설정 ===\n");
    if let Ok(entries) = fs::read_dir(&cfg_dir) {
        let confs: Vec<_> = entries.flatten()
            .filter(|e| e.path().extension().map(|x| x == "conf").unwrap_or(false))
            .collect();

        if confs.is_empty() {
            println!("등록된 설정이 없습니다.");
            println!("  mac run wireguard add <name> <path/to/conf>");
            return;
        }

        // Check which are active
        let active = cmd_stdout("wg", &["show", "interfaces"]);
        let active_list: Vec<&str> = active.split_whitespace().collect();

        for c in &confs {
            let name = c.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let running = active_list.contains(&name.as_str());
            println!("  {} {:<20} {}", if running { "✓" } else { " " }, name, c.path().display());
        }
    }
}

fn cmd_up(name: &str) {
    let conf = config_dir().join(format!("{}.conf", name));
    if !conf.exists() {
        println!("✗ {} 설정이 없습니다: {}", name, conf.display());
        return;
    }
    println!("WireGuard {} 연결 중...", name);
    let status = Command::new("sudo").args(["wg-quick", "up", name]).status();
    match status {
        Ok(s) if s.success() => println!("✓ {} 연결됨", name),
        _ => println!("✗ 연결 실패"),
    }
}

fn cmd_down(name: &str) {
    println!("WireGuard {} 정지 중...", name);
    let status = Command::new("sudo").args(["wg-quick", "down", name]).status();
    match status {
        Ok(s) if s.success() => println!("✓ {} 정지됨", name),
        _ => println!("✗ 정지 실패"),
    }
}

fn cmd_open() {
    let _ = Command::new("open").args(["-a", "WireGuard"]).output();
    println!("✓ WireGuard.app 실행");
}

fn cmd_add(name: &str, conf: &str) {
    let src = PathBuf::from(conf);
    if !src.exists() {
        println!("✗ 파일이 없습니다: {}", conf);
        return;
    }

    let cfg_dir = config_dir();
    fs::create_dir_all(&cfg_dir).ok();
    let dest = cfg_dir.join(format!("{}.conf", name));

    match fs::copy(&src, &dest) {
        Ok(_) => {
            // Set permissions 600
            let _ = Command::new("chmod").args(["600", &dest.to_string_lossy()]).output();
            println!("✓ {} 추가 완료 → {}", name, dest.display());
            println!("  연결: mac run wireguard up {}", name);
        }
        Err(e) => println!("✗ 복사 실패: {}", e),
    }
}
