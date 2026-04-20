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

use mac_common::{cmd, tui_spec::{self, TuiSpec}};

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
    let docker_installed = cmd::ok("which", &["docker"]);
    let orb_installed = cmd::ok("which", &["orbctl"]);
    let orb_running = orb_installed && cmd::stdout("orbctl", &["status"]).contains("Running");

    let usage_active = docker_installed;
    let usage_summary = if orb_running { "OrbStack 실행 중".to_string() }
        else if docker_installed { "Docker 설치됨".to_string() }
        else { "미설치".to_string() };

    TuiSpec::new("container")
        .refresh(15)
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("Docker CLI",
                if docker_installed { "✓ 설치됨" } else { "✗ 미설치" },
                if docker_installed { "ok" } else { "error" }),
            tui_spec::kv_item("OrbStack",
                if orb_installed { "✓ 설치됨" } else { "✗ 미설치" },
                if orb_installed { "ok" } else { "error" }),
            tui_spec::kv_item("OrbStack 실행",
                if orb_running { "✓ Running" } else { "✗ 정지" },
                if orb_running { "ok" } else { "warn" }),
        ])
        .buttons()
        .print();
}

fn cmd_status() {
    println!("=== Container 상태 ===\n");

    // OrbStack
    let orb_installed = cmd::ok("which", &["orbctl"]);
    if orb_installed {
        let ver = cmd::stdout("orbctl", &["version"]);
        let running = cmd::stdout("orbctl", &["status"]);
        let is_running = running.contains("Running");
        println!("[OrbStack] ✓ {} ({})", ver, if is_running { "✓ 실행 중" } else { "✗ 정지" });

        if is_running {
            // VMs
            let vms = cmd::stdout("orbctl", &["list"]);
            let vm_count = vms.lines().count().saturating_sub(1);
            println!("[VMs] {} 개", vm_count);

            // Docker containers
            let containers = cmd::stdout("docker", &["ps", "--format", "{{.Names}}"]);
            let running_count = containers.lines().filter(|l| !l.is_empty()).count();
            let all = cmd::stdout("docker", &["ps", "-a", "--format", "{{.Names}}"]);
            let all_count = all.lines().filter(|l| !l.is_empty()).count();
            println!("[Docker] {}/{} 실행 중", running_count, all_count);
        }
    } else {
        println!("[OrbStack] ✗ 미설치");
        println!("  → mai run container install-orbstack");
    }

    // Docker CLI
    let docker = cmd::ok("which", &["docker"]);
    println!("\n[docker CLI] {}", if docker { "✓ 설치됨" } else { "✗ 미설치" });

    // Docker Compose
    let compose = cmd::ok("docker", &["compose", "version"]);
    println!("[docker compose] {}", if compose { "✓ 설치됨" } else { "✗ 미설치" });
}

fn cmd_list() {
    if !cmd::ok("orbctl", &["status"]) || !cmd::stdout("orbctl", &["status"]).contains("Running") {
        println!("OrbStack이 실행 중이 아닙니다. mai run container up");
        return;
    }

    println!("=== Docker Containers ===\n");
    let out = cmd::stdout("docker", &["ps", "-a", "--format", "table {{.Names}}\t{{.Status}}\t{{.Image}}\t{{.Ports}}"]);
    println!("{}", out);
}

fn cmd_vms() {
    println!("=== OrbStack VMs ===\n");
    let out = cmd::output("orbctl", &["list"]);
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
    if cmd::ok("which", &["orbctl"]) {
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
