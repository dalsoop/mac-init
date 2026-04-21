use clap::{Parser, Subcommand};
use mac_common::{
    paths,
    tui_spec::{self, TuiSpec},
};
use mac_host_core::common;
use std::fs;
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
        #[arg(long, default_value = "pam")]
        realm: String,
        #[arg(long, default_value_t = 8006)]
        web_port: u16,
        #[arg(long)]
        password: Option<String>,
    },
    /// SSH 키 생성 + Proxmox에 공개키 등록
    SshSetup {
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
        Commands::Setup {
            host,
            user,
            realm,
            web_port,
            password,
        } => cmd_setup(&host, &user, &realm, web_port, password.as_deref()),
        Commands::SshSetup { password } => cmd_ssh_setup(password.as_deref()),
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

fn proxmox_realm() -> String {
    common::env_or("PROXMOX_REALM", "pam")
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

fn proxmox_login_user() -> String {
    format!("{}@{}", proxmox_user(), proxmox_realm())
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
#[derive(Clone, Debug)]
struct ApiSession {
    ticket: String,
    csrf: Option<String>,
    user: String,
}

fn api_login() -> Result<ApiSession, String> {
    let host = proxmox_host();
    let port = proxmox_web_port();
    let user = proxmox_login_user();
    let password = common::env_opt("PROXMOX_PASSWORD")
        .ok_or_else(|| "PROXMOX_PASSWORD 미설정".to_string())?;

    let url = format!("https://{}:{}/api2/json/access/ticket", host, port);
    let output = Command::new("curl")
        .args([
            "-sk",
            "--connect-timeout",
            "5",
            "--data-urlencode",
            &format!("username={user}"),
            "--data-urlencode",
            &format!("password={password}"),
            &url,
        ])
        .output()
        .map_err(|e| format!("curl 실행 실패: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("API 응답 파싱 실패: {}", e))?;

    if let Some(ticket) = json
        .get("data")
        .and_then(|d| d.get("ticket"))
        .and_then(|v| v.as_str())
    {
        let csrf = json
            .get("data")
            .and_then(|d| d.get("CSRFPreventionToken"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        return Ok(ApiSession {
            ticket: ticket.to_string(),
            csrf,
            user,
        });
    }

    let message = json
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("API 인증 실패")
        .trim()
        .to_string();
    Err(message)
}

fn api_get(path: &str, ticket: &str) -> Result<serde_json::Value, String> {
    let host = proxmox_host();
    let port = proxmox_web_port();
    let url = format!("https://{}:{}/api2/json{}", host, port, path);
    let cookie = format!("PVEAuthCookie={ticket}");

    let output = Command::new("curl")
        .args(["-sk", "--connect-timeout", "5", "-b", &cookie, &url])
        .output()
        .map_err(|e| format!("curl 실행 실패: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| format!("API 응답 파싱 실패: {}", e))
}

fn api_nodes(ticket: &str) -> Result<Vec<String>, String> {
    let json = api_get("/nodes", ticket)?;
    Ok(json
        .get("data")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("node").and_then(|v| v.as_str()))
        .map(ToString::to_string)
        .collect())
}

fn api_lxc_lines() -> Result<Vec<String>, String> {
    let session = api_login()?;
    let nodes = api_nodes(&session.ticket)?;
    let mut lines = Vec::new();

    for node in nodes {
        let json = api_get(&format!("/nodes/{node}/lxc"), &session.ticket)?;
        if let Some(items) = json.get("data").and_then(|v| v.as_array()) {
            for item in items {
                let vmid = item.get("vmid").and_then(|v| v.as_i64()).unwrap_or_default();
                let status = item
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                lines.push(format!("{:<5} {:<8} {:<20} {}", vmid, status, name, node));
            }
        }
    }

    Ok(lines)
}

fn ssh_key_path() -> PathBuf {
    PathBuf::from(paths::home()).join(".ssh/id_ed25519")
}

fn ssh_pub_key_path() -> PathBuf {
    PathBuf::from(paths::home()).join(".ssh/id_ed25519.pub")
}

fn proxmox_password(password: Option<&str>) -> Option<String> {
    password
        .map(ToString::to_string)
        .or_else(|| common::env_opt("PROXMOX_PASSWORD"))
}

fn shell_single_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn sshpass_exists() -> bool {
    Command::new("sshpass")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ensure_local_ssh_key() -> Result<bool, String> {
    let ssh_dir = PathBuf::from(paths::home()).join(".ssh");
    let key_path = ssh_key_path();
    if key_path.exists() {
        return Ok(false);
    }

    fs::create_dir_all(&ssh_dir)
        .map_err(|e| format!("~/.ssh 생성 실패: {}", e))?;

    let comment = common::env_opt("GIT_EMAIL")
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("USER").ok().map(|u| format!("{u}@mac-app-init")))
        .unwrap_or_else(|| "mac-app-init".to_string());

    let status = Command::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-C",
            &comment,
            "-f",
            &key_path.to_string_lossy(),
            "-N",
            "",
        ])
        .status()
        .map_err(|e| format!("ssh-keygen 실행 실패: {}", e))?;

    if status.success() {
        Ok(true)
    } else {
        Err("SSH 키 생성 실패".into())
    }
}

fn reset_known_host(host: &str) {
    let _ = Command::new("ssh-keygen").args(["-R", host]).status();
    let bracket_host = format!("[{host}]:22");
    let _ = Command::new("ssh-keygen").args(["-R", &bracket_host]).status();
}

fn install_pubkey_via_password(password: &str) -> Result<(), String> {
    let host = proxmox_host();
    let user = proxmox_user();
    let pub_key = fs::read_to_string(ssh_pub_key_path())
        .map_err(|e| format!("공개키 읽기 실패: {}", e))?
        .trim()
        .to_string();
    if pub_key.is_empty() {
        return Err("공개키가 비어 있습니다.".into());
    }

    reset_known_host(&host);

    let remote_cmd = format!(
        "mkdir -p ~/.ssh && chmod 700 ~/.ssh && touch ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && grep -qxF {0} ~/.ssh/authorized_keys || echo {0} >> ~/.ssh/authorized_keys",
        shell_single_quote(&pub_key)
    );

    let output = Command::new("sshpass")
        .args([
            "-p",
            password,
            "ssh",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=5",
            &format!("{user}@{host}"),
            &remote_cmd,
        ])
        .output()
        .map_err(|e| format!("sshpass/ssh 실행 실패: {}", e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Permission denied (publickey)") {
        return Err(format!(
            "대상 서버가 {}@{} 비밀번호 SSH 로그인을 허용하지 않습니다. 공개키를 서버에 먼저 등록하거나 SSH 설정(PasswordAuthentication/PermitRootLogin)을 확인해야 합니다.",
            user, host
        ));
    }

    if stderr.contains("Permission denied") {
        return Err("SSH 비밀번호 인증 실패입니다. PROXMOX_PASSWORD 또는 SSH 사용자 설정을 확인하세요.".into());
    }

    Err(format!("SSH 공개키 등록 실패: {}", stderr.trim()))
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

fn lxc_lines_with_source() -> Result<(Vec<String>, &'static str), String> {
    if ssh_login_ok() {
        return Ok((lxc_lines(), "ssh"));
    }
    api_lxc_lines().map(|lines| (lines, "api"))
}

fn cmd_status() {
    let host = proxmox_host();
    let user = proxmox_user();
    let web_port = proxmox_web_port();
    let web_ok = probe_tcp(&host, web_port);
    let ssh_port_ok = probe_tcp(&host, 22);
    let ssh_ok = ssh_login_ok();
    let api_session = api_login();
    let api_ok = api_session.is_ok();
    let api_user = api_session
        .as_ref()
        .map(|s| s.user.clone())
        .unwrap_or_else(|_| proxmox_login_user());
    let lxc_state = lxc_lines_with_source();

    println!("=== Proxmox 상태 ===\n");
    println!("[등록] {}", if proxmox_card_exists() { "✓ proxmox 카드" } else { "✗ env setup-proxmox 필요" });
    println!("[Web UI] {} {}", proxmox_url(), if web_ok { "✓ 연결 가능" } else { "✗ 연결 불가" });
    println!("[계정] {} {}", user, if proxmox_password_exists() { "✓ dotenvx 비번 있음" } else { "✗ 비번 없음" });
    println!("[API 로그인] {} {}", api_user, if api_ok { "✓ 가능" } else { "✗ 실패" });
    println!("[SSH 포트] {}:22 {}", host, if ssh_port_ok { "✓ 열림" } else { "✗ 닫힘" });
    println!("[SSH 로그인] {}", if ssh_ok { "✓ 키 기반 접속 가능" } else { "✗ 미설정/실패" });
    match lxc_state {
        Ok((lines, source)) => println!("[LXC] {} 개 ({})", lines.len(), source),
        Err(_) if api_ok => println!("[LXC] API 연결됨, 목록 조회 실패"),
        Err(_) => println!("[LXC] API 또는 SSH 로그인 필요"),
    }
}

fn cmd_setup(host: &str, user: &str, realm: &str, web_port: u16, password: Option<&str>) {
    let env_bin = env_domain_bin();
    let mut cmd = Command::new(&env_bin);
    cmd.args([
        "setup-proxmox",
        "--host",
        host,
        "--user",
        user,
        "--realm",
        realm,
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

    if password.is_some() {
        println!();
        match ssh_setup_result(password) {
            Ok(()) => {}
            Err(err) => {
                eprintln!("⚠ SSH 키 자동 등록 실패: {}", err);
                if api_login().is_ok() {
                    eprintln!("  API 로그인은 가능하므로 Proxmox 조회는 계속 사용할 수 있습니다.");
                } else {
                    eprintln!("  API 로그인도 실패하므로 이후 Proxmox 작업이 제한될 수 있습니다.");
                }
            }
        }
    } else {
        println!("ℹ SSH/LXC까지 쓰려면: mai run proxmox ssh-setup --password '...'");
    }
}

fn ssh_setup_result(password: Option<&str>) -> Result<(), String> {
    let host = proxmox_host();
    let user = proxmox_user();

    println!("=== Proxmox SSH 설정 ===\n");

    match ensure_local_ssh_key() {
        Ok(true) => println!("✓ 로컬 SSH 키 생성: {}", ssh_key_path().display()),
        Ok(false) => println!("✓ 로컬 SSH 키 존재: {}", ssh_key_path().display()),
        Err(err) => return Err(err),
    }

    if ssh_login_ok() {
        println!("✓ 이미 SSH 키 기반 접속 가능: {}@{}", user, host);
        return Ok(());
    }

    if !sshpass_exists() {
        return Err("sshpass가 필요합니다. 먼저 `mai run bootstrap install`을 실행하세요.".into());
    }

    let Some(password) = proxmox_password(password) else {
        return Err("PROXMOX_PASSWORD가 없어서 자동 등록을 할 수 없습니다. → mai run proxmox ssh-setup --password '...'".into());
    };

    println!("→ {}@{}에 공개키 등록 중...", user, host);
    if let Err(err) = install_pubkey_via_password(&password) {
        if ssh_login_ok() {
            println!("✓ SSH 키 등록 완료");
            return Ok(());
        }
        return Err(format!("{} (공개키: {})", err, ssh_pub_key_path().display()));
    }

    if ssh_login_ok() {
        println!("✓ SSH 키 등록 완료");
        Ok(())
    } else {
        Err("공개키 등록 후에도 SSH 접속 검증에 실패했습니다.".into())
    }
}

fn cmd_ssh_setup(password: Option<&str>) {
    if let Err(err) = ssh_setup_result(password) {
        eprintln!("✗ {}", err);
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
    match lxc_lines_with_source() {
        Ok((lines, source)) => {
            if lines.is_empty() {
                println!("LXC 없음 ({})", source);
                return;
            }
            println!("=== Proxmox LXC ({}) ===\n", source);
            for line in lines {
                println!("  {}", line);
            }
        }
        Err(_) => {
            eprintln!("✗ SSH 또는 API 로그인 경로가 필요합니다.");
            std::process::exit(1);
        }
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
    let api_session = api_login();
    let api_ok = api_session.is_ok();
    let api_user = api_session
        .as_ref()
        .map(|s| s.user.clone())
        .unwrap_or_else(|_| proxmox_login_user());
    let api_csrf = api_session
        .as_ref()
        .ok()
        .and_then(|s| s.csrf.as_ref())
        .is_some();
    let (lxc, lxc_source) = match lxc_lines_with_source() {
        Ok((lines, source)) => (lines, source),
        Err(_) => (Vec::new(), "none"),
    };

    let usage_active = proxmox_card_exists();
    let usage_summary = if usage_active {
        let auth = if ssh_ok {
            "ssh"
        } else if api_ok {
            "api"
        } else {
            "auth down"
        };
        format!("{} / web {} / {}", proxmox_url(), if web_ok { "ok" } else { "down" }, auth)
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
            tui_spec::kv_item(
                "등록",
                if proxmox_card_exists() { "✓ proxmox 카드" } else { "✗ setup 필요" },
                if proxmox_card_exists() { "ok" } else { "error" },
            ),
            tui_spec::kv_item("클러스터", &node_info,
                if !nodes.is_empty() { "ok" } else { "error" }),
            tui_spec::kv_item("Web UI", &proxmox_url(), if web_ok { "ok" } else { "error" }),
            tui_spec::kv_item("계정", &user, if proxmox_password_exists() { "ok" } else { "warn" }),
            tui_spec::kv_item("API 로그인", &api_user, if api_ok { "ok" } else { "warn" }),
            tui_spec::kv_item("API 쓰기", if api_csrf { "✓ 가능" } else { "✗ 불가" }, if api_csrf { "ok" } else { "warn" }),
            tui_spec::kv_item("SSH", &format!("{}@{}:22", user, host),
                if ssh_ok { "ok" } else { "warn" }),
            tui_spec::kv_item("LXC",
                &if ssh_ok {
                    format!("{}/{} running", lxc_running, lxc_total)
                } else if api_ok {
                    format!("{} visible via {}", lxc.len(), lxc_source)
                } else {
                    "API/SSH 로그인 필요".into()
                },
                if ssh_ok || api_ok { "ok" } else { "warn" }),
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
        .buttons_custom(
            "빠른 실행",
            vec![
                serde_json::json!({ "label": "기본 등록", "command": "setup", "key": "s" }),
                serde_json::json!({ "label": "웹 UI 열기", "command": "open", "key": "o" }),
                serde_json::json!({ "label": "SSH 키 등록", "command": "ssh-setup", "key": "k" }),
                serde_json::json!({ "label": "LXC 목록", "command": "lxc-list", "key": "l" }),
                serde_json::json!({ "label": "호스트 SSH", "command": "ssh", "key": "h" }),
            ],
        )
        .text("안내",
            "  mai run proxmox setup --realm pam --password ...\n  \
             SSH가 막혀도 API 인증이 맞으면 LXC 목록은 API fallback 으로 조회합니다.\n  \
             mai run proxmox ssh              호스트 SSH 접속\n  \
             mai run proxmox lxc-shell 50063   LXC 셸 접속\n  \
             mai run proxmox lxc-exec 50063 ls 명령 실행\n  \
             mai run proxmox lxc-start 50063   LXC 시작\n  \
             mai run proxmox lxc-stop 50063    LXC 정지")
        .print();
}
