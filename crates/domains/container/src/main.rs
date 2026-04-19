use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-container")]
#[command(about = "Docker/OrbStack 컨테이너 + VM 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 확인
    Status,
    /// Docker 컨테이너 목록
    List,
    /// OrbStack VM 목록
    Vms,
    /// 컨테이너/VM 시작
    Start { name: String },
    /// 컨테이너/VM 정지
    Stop { name: String },
    /// 컨테이너/VM 재시작
    Restart { name: String },
    /// OrbStack 설치
    InstallOrbstack,
    /// OrbStack 시작
    Up,
    /// OrbStack 정지
    Down,
    /// Docker 로그
    Logs { name: String },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn cmd_ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).output().map(|o| o.status.success()).unwrap_or(false)
}

fn cmd_out(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output()
        .map(|o| format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
        .unwrap_or_else(|e| format!("Error: {}", e))
}

fn cmd_stdout(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::List => cmd_list(),
        Commands::Vms => cmd_vms(),
        Commands::Start { name } => cmd_start(&name),
        Commands::Stop { name } => cmd_stop(&name),
        Commands::Restart { name } => cmd_restart(&name),
        Commands::InstallOrbstack => cmd_install_orbstack(),
        Commands::Up => cmd_up(),
        Commands::Down => cmd_down(),
        Commands::Logs { name } => cmd_logs(&name),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let docker_installed = cmd_ok("which", &["docker"]);
    let orb_installed = cmd_ok("which", &["orbctl"]);
    let orb_running = orb_installed && cmd_stdout("orbctl", &["status"]).contains("Running");

    let spec = serde_json::json!({
        "tab": { "label_ko": "컨테이너", "label": "Container", "icon": "📦" },
        "group": "dev",        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    {
                        "key": "Docker CLI",
                        "value": if docker_installed { "✓ 설치됨" } else { "✗ 미설치" },
                        "status": if docker_installed { "ok" } else { "error" }
                    },
                    {
                        "key": "OrbStack",
                        "value": if orb_installed { "✓ 설치됨" } else { "✗ 미설치" },
                        "status": if orb_installed { "ok" } else { "error" }
                    },
                    {
                        "key": "OrbStack 실행",
                        "value": if orb_running { "✓ Running" } else { "✗ 정지" },
                        "status": if orb_running { "ok" } else { "warn" }
                    }
                ]
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label_ko": "컨테이너", "label": "Status (상태)", "command": "status", "key": "s" },
                    { "label_ko": "컨테이너", "label": "List (컨테이너 목록)", "command": "list", "key": "l" },
                    { "label_ko": "컨테이너", "label": "Vms (VM 목록)", "command": "vms", "key": "v" },
                    { "label_ko": "컨테이너", "label": "Up (OrbStack 시작)", "command": "up", "key": "u" },
                    { "label_ko": "컨테이너", "label": "Down (OrbStack 정지)", "command": "down", "key": "d" },
                    { "label_ko": "컨테이너", "label": "Install OrbStack", "command": "install-orbstack", "key": "i" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}

fn cmd_status() {
    println!("=== Container 상태 ===\n");

    // OrbStack
    let orb_installed = cmd_ok("which", &["orbctl"]);
    if orb_installed {
        let ver = cmd_stdout("orbctl", &["version"]);
        let running = cmd_stdout("orbctl", &["status"]);
        let is_running = running.contains("Running");
        println!("[OrbStack] ✓ {} ({})", ver, if is_running { "✓ 실행 중" } else { "✗ 정지" });

        if is_running {
            // VMs
            let vms = cmd_stdout("orbctl", &["list"]);
            let vm_count = vms.lines().count().saturating_sub(1);
            println!("[VMs] {} 개", vm_count);

            // Docker containers
            let containers = cmd_stdout("docker", &["ps", "--format", "{{.Names}}"]);
            let running_count = containers.lines().filter(|l| !l.is_empty()).count();
            let all = cmd_stdout("docker", &["ps", "-a", "--format", "{{.Names}}"]);
            let all_count = all.lines().filter(|l| !l.is_empty()).count();
            println!("[Docker] {}/{} 실행 중", running_count, all_count);
        }
    } else {
        println!("[OrbStack] ✗ 미설치");
        println!("  → mac run container install-orbstack");
    }

    // Docker CLI
    let docker = cmd_ok("which", &["docker"]);
    println!("\n[docker CLI] {}", if docker { "✓ 설치됨" } else { "✗ 미설치" });

    // Docker Compose
    let compose = cmd_ok("docker", &["compose", "version"]);
    println!("[docker compose] {}", if compose { "✓ 설치됨" } else { "✗ 미설치" });
}

fn cmd_list() {
    if !cmd_ok("orbctl", &["status"]) || !cmd_stdout("orbctl", &["status"]).contains("Running") {
        println!("OrbStack이 실행 중이 아닙니다. mac run container up");
        return;
    }

    println!("=== Docker Containers ===\n");
    let out = cmd_stdout("docker", &["ps", "-a", "--format", "table {{.Names}}\t{{.Status}}\t{{.Image}}\t{{.Ports}}"]);
    println!("{}", out);
}

fn cmd_vms() {
    println!("=== OrbStack VMs ===\n");
    let out = cmd_out("orbctl", &["list"]);
    print!("{}", out);
}

fn cmd_start(name: &str) {
    // Try docker first, then orbctl
    let docker_ok = Command::new("docker").args(["start", name]).output()
        .map(|o| o.status.success()).unwrap_or(false);
    if docker_ok {
        println!("✓ {} 시작 (docker)", name);
    } else {
        let orb_ok = Command::new("orbctl").args(["start", name]).output()
            .map(|o| o.status.success()).unwrap_or(false);
        if orb_ok {
            println!("✓ {} 시작 (orbstack)", name);
        } else {
            println!("✗ {} 시작 실패", name);
        }
    }
}

fn cmd_stop(name: &str) {
    let docker_ok = Command::new("docker").args(["stop", name]).output()
        .map(|o| o.status.success()).unwrap_or(false);
    if docker_ok {
        println!("✓ {} 정지 (docker)", name);
    } else {
        let orb_ok = Command::new("orbctl").args(["stop", name]).output()
            .map(|o| o.status.success()).unwrap_or(false);
        if orb_ok {
            println!("✓ {} 정지 (orbstack)", name);
        } else {
            println!("✗ {} 정지 실패", name);
        }
    }
}

fn cmd_restart(name: &str) {
    let docker_ok = Command::new("docker").args(["restart", name]).output()
        .map(|o| o.status.success()).unwrap_or(false);
    if docker_ok {
        println!("✓ {} 재시작 (docker)", name);
    } else {
        let orb_ok = Command::new("orbctl").args(["restart", name]).output()
            .map(|o| o.status.success()).unwrap_or(false);
        if orb_ok {
            println!("✓ {} 재시작 (orbstack)", name);
        } else {
            println!("✗ {} 재시작 실패", name);
        }
    }
}

fn cmd_up() {
    println!("OrbStack 시작 중...");
    let ok = Command::new("orbctl").args(["start"]).output()
        .map(|o| o.status.success()).unwrap_or(false);
    if ok {
        println!("✓ OrbStack 시작됨");
    } else {
        // Try open app
        let _ = Command::new("open").args(["-a", "OrbStack"]).output();
        println!("✓ OrbStack 앱 시작");
    }
}

fn cmd_down() {
    println!("OrbStack 정지 중...");
    let ok = Command::new("orbctl").args(["stop"]).output()
        .map(|o| o.status.success()).unwrap_or(false);
    println!("{}", if ok { "✓ OrbStack 정지됨" } else { "✗ 정지 실패" });
}

fn cmd_install_orbstack() {
    if cmd_ok("which", &["orbctl"]) {
        println!("✓ OrbStack 이미 설치됨");
        return;
    }
    println!("OrbStack 설치 중...");
    let ok = Command::new("brew").args(["install", "--cask", "orbstack"]).status()
        .map(|s| s.success()).unwrap_or(false);
    println!("{}", if ok { "✓ OrbStack 설치 완료" } else { "✗ 설치 실패" });
}

fn cmd_logs(name: &str) {
    let _ = Command::new("docker").args(["logs", "--tail", "50", name]).status();
}
