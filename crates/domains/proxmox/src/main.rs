use clap::{Parser, Subcommand};
use mac_common::{
    paths,
    tui_spec::{self, TuiSpec},
};
use mac_host_core::common;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "mac-domain-proxmox")]
#[command(about = "Proxmox VE 웹 UI + 상태 확인")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 상태 확인
    Status,
    /// Proxmox 기본 연결정보 등록 (.env + env 카드)
    Setup {
        #[arg(long, default_value = "192.168.2.50")]
        host: String,
        #[arg(long, default_value = "root")]
        user: String,
        #[arg(long, default_value_t = 8006)]
        web_port: u16,
        #[arg(long)]
        password: Option<String>,
    },
    /// Proxmox 웹 UI 열기
    Open,
    /// LXC 목록
    LxcList,
    /// VM 목록
    VmList,
    /// LXC 셸 접속 (pct enter)
    LxcShell { vmid: String },
    /// LXC에서 명령 실행 (pct exec)
    LxcExec { vmid: String, cmd: Vec<String> },
    /// LXC 시작
    LxcStart { vmid: String },
    /// LXC 정지
    LxcStop { vmid: String },
    /// Proxmox 호스트 SSH 셸
    Ssh,
    /// TUI 스펙 (JSON)
    TuiSpec,
}

fn main() {
    common::load_env();

    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Setup { host, user, web_port, password } =>
            cmd_setup(&host, &user, web_port, password.as_deref()),
        Commands::Open => cmd_open(),
        Commands::LxcList => cmd_lxc_list(),
        Commands::VmList => cmd_vm_list(),
        Commands::LxcShell { vmid } => cmd_lxc_shell(&vmid),
        Commands::LxcExec { vmid, cmd } => cmd_lxc_exec(&vmid, &cmd),
        Commands::LxcStart { vmid } => cmd_lxc_start(&vmid),
        Commands::LxcStop { vmid } => cmd_lxc_stop(&vmid),
        Commands::Ssh => cmd_ssh(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn load_card() -> Option<serde_json::Value> {
    let path = PathBuf::from(paths::home()).join(".mac-app-init/cards/proxmox.json");
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn proxmox_host() -> String {
    load_card().and_then(|c| c.get("host").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| "192.168.2.50".into())
}

fn proxmox_user() -> String {
    load_card().and_then(|c| c.get("user").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| "root".into())
}

fn proxmox_web_port() -> u16 {
    common::env_or("PROXMOX_WEB_PORT", "8006")
        .parse()
        .unwrap_or(8006)
}

fn proxmox_password_exists() -> bool {
    common::env_opt("PROXMOX_PASSWORD").is_some()
}

fn proxmox_url() -> String {
    format!("https://{}:{}", proxmox_host(), proxmox_web_port())
}

fn proxmox_card_exists() -> bool {
    PathBuf::from(paths::home())
        .join(".mac-app-init/cards/proxmox.json")
        .exists()
}

fn env_domain_bin() -> PathBuf {
    let candidates = [
        PathBuf::from(paths::home()).join(".mac-app-init/domains/mac-domain-env"),
        PathBuf::from("target/debug/mac-domain-env"),
        PathBuf::from("target/release/mac-domain-env"),
    ];
    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }
    PathBuf::from("mac-domain-env")
}

fn probe_tcp(host: &str, port: u16) -> bool {
    let addr = format!("{host}:{port}");
    if let Ok(mut iter) = addr.to_socket_addrs() {
        if let Some(sock) = iter.next() {
            return std::net::TcpStream::connect_timeout(&sock, Duration::from_secs(2)).is_ok();
        }
    }
    false
}

fn ssh_login_ok() -> bool {
    let host = proxmox_host();
    let user = proxmox_user();
    let (ok, _) = common::ssh_cmd(&host, &user, "echo ok");
    ok
}

/// 클러스터 노드 목록 (pvesh).
fn cluster_nodes() -> Vec<(String, String)> {
    let host = proxmox_host();
    let user = proxmox_user();
    let (ok, output) = common::ssh_cmd(&host, &user,
        "pvesh get /nodes --output-format json 2>/dev/null");
    if !ok { return vec![("pve".into(), "unknown".into())]; }
    let nodes: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap_or_default();
    nodes.iter().map(|n| {
        let name = n.get("node").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let status = n.get("status").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        (name, status)
    }).collect()
}

/// 클러스터 전체 LXC (노드별). 반환: (node, vmid, status, name)
fn all_lxc() -> Vec<(String, String, String, String)> {
    let host = proxmox_host();
    let user = proxmox_user();
    let nodes = cluster_nodes();
    let mut result = Vec::new();
    for (node, _) in &nodes {
        let cmd = format!("pvesh get /nodes/{}/lxc --output-format json 2>/dev/null", node);
        let (ok, output) = common::ssh_cmd(&host, &user, &cmd);
        if !ok { continue; }
        let ctrs: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap_or_default();
        for c in ctrs {
            let vmid = c.get("vmid").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_default();
            let status = c.get("status").and_then(|v| v.as_str()).unwrap_or("?").to_string();
            let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
            result.push((node.clone(), vmid, status, name));
        }
    }
    result.sort_by(|a, b| a.1.cmp(&b.1));
    result
}

/// 클러스터 전체 VM (노드별).
fn all_vms() -> Vec<(String, String, String, String)> {
    let host = proxmox_host();
    let user = proxmox_user();
    let nodes = cluster_nodes();
    let mut result = Vec::new();
    for (node, _) in &nodes {
        let cmd = format!("pvesh get /nodes/{}/qemu --output-format json 2>/dev/null", node);
        let (ok, output) = common::ssh_cmd(&host, &user, &cmd);
        if !ok { continue; }
        let vms: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap_or_default();
        for v in vms {
            let vmid = v.get("vmid").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_default();
            let status = v.get("status").and_then(|v| v.as_str()).unwrap_or("?").to_string();
            let name = v.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
            result.push((node.clone(), vmid, status, name));
        }
    }
    result.sort_by(|a, b| a.1.cmp(&b.1));
    result
}

fn lxc_lines() -> Vec<String> {
    all_lxc().iter().map(|(node, vmid, status, name)| {
        format!("{:<8} {:<10} {:<10} {}", vmid, status, node, name)
    }).collect()
}

fn cmd_status() {
    let host = proxmox_host();
    let user = proxmox_user();
    let web_port = proxmox_web_port();
    let web_ok = probe_tcp(&host, web_port);
    let ssh_port_ok = probe_tcp(&host, 22);
    let ssh_ok = ssh_login_ok();
    let lxc = if ssh_ok { lxc_lines().len() } else { 0 };

    println!("=== Proxmox 상태 ===\n");
    println!("[등록] {}", if proxmox_card_exists() { "✓ proxmox 카드" } else { "✗ env setup-proxmox 필요" });
    println!("[Web UI] {} {}", proxmox_url(), if web_ok { "✓ 연결 가능" } else { "✗ 연결 불가" });
    println!("[계정] {} {}", user, if proxmox_password_exists() { "✓ dotenvx 비번 있음" } else { "✗ 비번 없음" });
    println!("[SSH 포트] {}:22 {}", host, if ssh_port_ok { "✓ 열림" } else { "✗ 닫힘" });
    println!("[SSH 로그인] {}", if ssh_ok { "✓ 키 기반 접속 가능" } else { "✗ 미설정/실패" });
    if ssh_ok {
        println!("[LXC] {} 개", lxc);
    } else {
        println!("[LXC] SSH 로그인 필요");
    }
}

fn cmd_setup(host: &str, user: &str, web_port: u16, password: Option<&str>) {
    let env_bin = env_domain_bin();
    let mut cmd = Command::new(&env_bin);
    cmd.args([
        "setup-proxmox",
        "--host",
        host,
        "--user",
        user,
        "--web-port",
        &web_port.to_string(),
    ]);
    if let Some(password) = password {
        cmd.args(["--password", password]);
    }
    let out = cmd.output().unwrap_or_else(|e| {
        eprintln!("✗ env 도메인 실행 실패: {}", e);
        std::process::exit(1);
    });
    print!("{}", String::from_utf8_lossy(&out.stdout));
    eprint!("{}", String::from_utf8_lossy(&out.stderr));
    if !out.status.success() {
        std::process::exit(1);
    }
}

fn cmd_open() {
    let url = proxmox_url();
    let out = Command::new("open")
        .arg(&url)
        .output()
        .unwrap_or_else(|e| {
            eprintln!("✗ open 실행 실패: {}", e);
            std::process::exit(1);
        });
    if !out.status.success() {
        eprintln!("✗ {}", String::from_utf8_lossy(&out.stderr).trim());
        std::process::exit(1);
    }
    println!("✓ 열기: {}", url);
}

fn cmd_lxc_list() {
    if !ssh_login_ok() {
        eprintln!("✗ SSH 키 기반 접속이 필요합니다. 현재 proxmox 웹 등록만 된 상태입니다.");
        std::process::exit(1);
    }
    let lines = lxc_lines();
    if lines.is_empty() {
        println!("LXC 없음");
        return;
    }
    println!("=== Proxmox LXC ===\n");
    for line in lines {
        println!("  {}", line);
    }
}

fn vm_lines() -> Vec<String> {
    all_vms().iter().map(|(node, vmid, status, name)| {
        format!("{:<8} {:<10} {:<10} {}", vmid, status, node, name)
    }).collect()
}

fn cmd_vm_list() {
    if !ssh_login_ok() { eprintln!("✗ SSH 접속 불가"); std::process::exit(1); }
    let lines = vm_lines();
    if lines.is_empty() { println!("VM 없음"); return; }
    println!("=== Proxmox VM ===\n");
    for line in lines { println!("  {}", line); }
}

fn ssh_target() -> String {
    format!("{}@{}", proxmox_user(), proxmox_host())
}

/// 이름 또는 VMID로 LXC VMID 찾기 (클러스터 전체 검색)
fn resolve_vmid(name_or_id: &str) -> String {
    if name_or_id.chars().all(|c| c.is_ascii_digit()) {
        return name_or_id.to_string();
    }
    for (_, vmid, _, name) in all_lxc() {
        if name == name_or_id {
            return vmid;
        }
    }
    eprintln!("✗ LXC '{}' 를 찾을 수 없습니다.", name_or_id);
    std::process::exit(1);
}

/// VMID가 어느 노드에 있는지 찾기 (pct enter는 해당 노드에서 실행해야 함)
fn find_node_for_vmid(vmid: &str) -> Option<String> {
    for (node, vid, _, _) in all_lxc() {
        if vid == vmid { return Some(node); }
    }
    None
}

fn cmd_lxc_shell(vmid: &str) {
    let vmid = resolve_vmid(vmid);
    let target = ssh_target();
    // 노드가 로컬(pve)이면 pct enter, 원격이면 해당 노드 ssh 경유
    let node = find_node_for_vmid(&vmid);
    let cmd = match node.as_deref() {
        Some("pve") | None => format!("pct enter {}", vmid),
        Some(remote) => format!("ssh -t {} 'pct enter {}'", remote, vmid),
    };
    println!("LXC {} 셸 접속 중... ({})", vmid, node.as_deref().unwrap_or("local"));
    let _ = Command::new("ssh")
        .args(["-t", &target, &cmd])
        .status();
}

fn cmd_lxc_exec(vmid: &str, cmd: &[String]) {
    let vmid = resolve_vmid(vmid);
    let target = ssh_target();
    let node = find_node_for_vmid(&vmid);
    let pct_cmd = format!("pct exec {} -- {}", vmid, cmd.join(" "));
    let remote_cmd = match node.as_deref() {
        Some("pve") | None => pct_cmd,
        Some(remote) => format!("ssh {} '{}'", remote, pct_cmd),
    };
    let out = Command::new("ssh").args([&target, &remote_cmd]).output();
    match out {
        Ok(o) => {
            print!("{}", String::from_utf8_lossy(&o.stdout));
            eprint!("{}", String::from_utf8_lossy(&o.stderr));
            std::process::exit(o.status.code().unwrap_or(1));
        }
        Err(e) => { eprintln!("✗ {}", e); std::process::exit(1); }
    }
}

fn cmd_lxc_start(vmid: &str) {
    let vmid = resolve_vmid(vmid);
    let node = find_node_for_vmid(&vmid);
    let pct = format!("pct start {}", vmid);
    let cmd = match node.as_deref() {
        Some("pve") | None => pct,
        Some(remote) => format!("ssh {} '{}'", remote, pct),
    };
    let (ok, out) = common::ssh_cmd(&proxmox_host(), &proxmox_user(), &cmd);
    if ok { println!("✓ LXC {} 시작 ({})", vmid, node.as_deref().unwrap_or("local")); } else { eprintln!("✗ {}", out); }
}

fn cmd_lxc_stop(vmid: &str) {
    let vmid = resolve_vmid(vmid);
    let node = find_node_for_vmid(&vmid);
    let pct = format!("pct stop {}", vmid);
    let cmd = match node.as_deref() {
        Some("pve") | None => pct,
        Some(remote) => format!("ssh {} '{}'", remote, pct),
    };
    let (ok, out) = common::ssh_cmd(&proxmox_host(), &proxmox_user(), &cmd);
    if ok { println!("✓ LXC {} 정지 ({})", vmid, node.as_deref().unwrap_or("local")); } else { eprintln!("✗ {}", out); }
}

fn cmd_ssh() {
    let target = ssh_target();
    println!("Proxmox SSH 접속: {}", target);
    let _ = Command::new("ssh").arg("-t").arg(&target).status();
}

fn print_tui_spec() {
    let host = proxmox_host();
    let user = proxmox_user();
    let web_port = proxmox_web_port();
    let web_ok = probe_tcp(&host, web_port);
    let ssh_port_ok = probe_tcp(&host, 22);
    let ssh_ok = ssh_login_ok();
    let lxc = if ssh_ok { lxc_lines() } else { Vec::new() };

    let usage_active = proxmox_card_exists();
    let usage_summary = if usage_active {
        format!("{} / web {}", proxmox_url(), if web_ok { "ok" } else { "down" })
    } else {
        "미등록".to_string()
    };

    // 클러스터 전체 데이터
    let lxc_all = if ssh_ok { all_lxc() } else { Vec::new() };
    let vm_all = if ssh_ok { all_vms() } else { Vec::new() };
    let nodes = if ssh_ok { cluster_nodes() } else { Vec::new() };

    let lxc_rows: Vec<serde_json::Value> = lxc_all.iter()
        .map(|(node, vmid, status, name)| serde_json::json!([vmid, status, node, name]))
        .collect();

    let vm_rows: Vec<serde_json::Value> = vm_all.iter()
        .map(|(node, vmid, status, name)| serde_json::json!([vmid, status, node, name]))
        .collect();

    let lxc_running = lxc_all.iter().filter(|c| c.2 == "running").count();
    let lxc_total = lxc_all.len();
    let node_info = nodes.iter()
        .map(|(n, s)| format!("{} ({})", n, s))
        .collect::<Vec<_>>().join(", ");

    TuiSpec::new("proxmox")
        .refresh(30)
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("클러스터", &node_info,
                if !nodes.is_empty() { "ok" } else { "error" }),
            tui_spec::kv_item("Web UI", &proxmox_url(), if web_ok { "ok" } else { "error" }),
            tui_spec::kv_item("SSH", &format!("{}@{}:22", user, host),
                if ssh_ok { "ok" } else { "warn" }),
            tui_spec::kv_item("LXC",
                &format!("{}/{} running", lxc_running, lxc_total),
                if lxc_running > 0 { "ok" } else { "warn" }),
            tui_spec::kv_item("VM",
                &format!("{}", vm_all.len()),
                "ok"),
        ])
        .table("LXC 컨테이너",
            vec!["VMID", "STATUS", "NODE", "NAME"],
            lxc_rows)
        .table("VM",
            vec!["VMID", "STATUS", "NODE", "NAME"],
            vm_rows)
        .buttons()
        .text("안내",
            "  mai run proxmox ssh              호스트 SSH 접속\n  \
             mai run proxmox lxc-shell 50063   LXC 셸 접속\n  \
             mai run proxmox lxc-exec 50063 ls 명령 실행\n  \
             mai run proxmox lxc-start 50063   LXC 시작\n  \
             mai run proxmox lxc-stop 50063    LXC 정지")
        .print();
}
