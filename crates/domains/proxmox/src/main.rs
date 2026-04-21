use clap::{Parser, Subcommand};
use mac_common::{
    paths,
    tui_spec::{self, TuiSpec},
};
use mac_host_core::common;
use serde::Deserialize;
use std::fs;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "mac-domain-proxmox")]
#[command(about = "Proxmox VE 웹 UI + 상태 확인")]
struct Cli {
    #[arg(long, global = true, default_value = "proxmox50")]
    card: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Deserialize)]
struct CardBindMount {
    lxc: String,
    source: String,
    target: String,
    #[serde(default)]
    readonly: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ProxmoxCard {
    name: String,
    host: String,
    user: String,
    #[serde(default)]
    web_port: u16,
    #[serde(default)]
    bind_mounts: Vec<CardBindMount>,
}

fn active_proxmox_card() -> Option<ProxmoxCard> {
    let cards = load_proxmox_cards();
    if let Ok(selected) = std::env::var("MAI_PROXMOX_CARD") {
        if let Some(card) = cards.iter().find(|card| card.name == selected) {
            return Some(card.clone());
        }
    }
    cards
        .iter()
        .find(|card| card.name == "proxmox50")
        .cloned()
        .or_else(|| cards.into_iter().next())
}

fn active_proxmox_card_name() -> String {
    active_proxmox_card()
        .map(|c| c.name)
        .unwrap_or_else(|| "proxmox50".into())
}

fn active_proxmox_password_key() -> String {
    format!("{}_PASSWORD", active_proxmox_card_name().to_uppercase())
}

fn active_proxmox_web_port_key() -> String {
    format!("{}_WEB_PORT", active_proxmox_card_name().to_uppercase())
}

fn active_proxmox_host_key() -> String {
    format!("{}_HOST", active_proxmox_card_name().to_uppercase())
}

fn active_proxmox_user_key() -> String {
    format!("{}_USER", active_proxmox_card_name().to_uppercase())
}

fn active_proxmox_default_host() -> String {
    active_proxmox_card()
        .map(|c| c.host)
        .unwrap_or_else(|| "192.168.2.50".into())
}

fn active_proxmox_default_user() -> String {
    active_proxmox_card()
        .map(|c| c.user)
        .unwrap_or_else(|| "root".into())
}

fn active_proxmox_default_port() -> u16 {
    22
}

fn active_proxmox_default_web_port() -> u16 {
    active_proxmox_card()
        .map(|c| if c.web_port > 0 { c.web_port } else { 8006 })
        .unwrap_or(8006)
}

fn proxmox_host() -> String {
    common::env_or(&active_proxmox_host_key(), &active_proxmox_default_host())
}

fn proxmox_user() -> String {
    common::env_or(&active_proxmox_user_key(), &active_proxmox_default_user())
}

fn proxmox_web_port() -> u16 {
    common::env_or(
        &active_proxmox_web_port_key(),
        &active_proxmox_default_web_port().to_string(),
    )
        .parse()
        .unwrap_or(active_proxmox_default_web_port())
}

fn proxmox_password_exists() -> bool {
    common::env_opt(&active_proxmox_password_key()).is_some()
}

fn proxmox_url() -> String {
    format!("https://{}:{}", proxmox_host(), proxmox_web_port())
}

fn proxmox_card_exists() -> bool {
    !load_proxmox_cards().is_empty()
}

#[derive(Debug, Clone)]
struct BindMountRule {
    lxc: String,
    source: String,
    target: String,
    readonly: bool,
}

fn load_bind_mount_rules() -> Vec<BindMountRule> {
    let active = active_proxmox_card_name();
    load_proxmox_cards()
        .into_iter()
        .filter(|card| card.name == active)
        .flat_map(|card| {
            card.bind_mounts.into_iter().map(|bind| BindMountRule {
                lxc: bind.lxc,
                source: bind.source,
                target: bind.target,
                readonly: bind.readonly,
            })
        })
        .collect()
}

fn save_bind_mount_rules(rules: &[BindMountRule]) -> Result<(), String> {
    let active = active_proxmox_card_name();
    let path = paths::ssot_cards_dir().join(format!("{}.json", active));
    let content = fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("{}: {}", path.display(), e))?;
    let obj = json
        .as_object_mut()
        .ok_or_else(|| format!("{} 카드 JSON object 아님", active))?;
    obj.insert(
        "bind_mounts".into(),
        serde_json::json!(rules
            .iter()
            .map(|rule| serde_json::json!({
                "lxc": rule.lxc,
                "source": rule.source,
                "target": rule.target,
                "readonly": rule.readonly,
            }))
            .collect::<Vec<_>>()),
    );
    fs::write(
        &path,
        serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
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
    /// 선언된 bind mount 목록
    BindList,
    /// LXC bind mount 선언 추가
    BindAdd {
        lxc: String,
        source: String,
        target: String,
        #[arg(long)]
        readonly: bool,
    },
    /// LXC bind mount 선언 제거
    BindRemove {
        lxc: String,
        target: String,
    },
    /// 선언된 bind mount를 Proxmox LXC 설정에 동기화
    BindSync {
        /// 특정 LXC 또는 VMID만 동기화
        target: Option<String>,
    },
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
    unsafe {
        std::env::set_var("MAI_PROXMOX_CARD", &cli.card);
    }
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Setup {
            host,
            user,
            web_port,
            password,
        } => cmd_setup(&host, &user, web_port, password.as_deref()),
        Commands::Open => cmd_open(),
        Commands::LxcList => cmd_lxc_list(),
        Commands::BindList => cmd_bind_list(),
        Commands::BindAdd {
            lxc,
            source,
            target,
            readonly,
        } => cmd_bind_add(&lxc, &source, &target, readonly),
        Commands::BindRemove { lxc, target } => cmd_bind_remove(&lxc, &target),
        Commands::BindSync { target } => cmd_bind_sync(target.as_deref()),
        Commands::VmList => cmd_vm_list(),
        Commands::LxcShell { vmid } => cmd_lxc_shell(&vmid),
        Commands::LxcExec { vmid, cmd } => cmd_lxc_exec(&vmid, &cmd),
        Commands::LxcStart { vmid } => cmd_lxc_start(&vmid),
        Commands::LxcStop { vmid } => cmd_lxc_stop(&vmid),
        Commands::Ssh => cmd_ssh(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn load_proxmox_cards() -> Vec<ProxmoxCard> {
    let dir = paths::ssot_cards_dir();
    if !dir.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Ok(it) = fs::read_dir(dir) {
        for e in it.filter_map(|x| x.ok()) {
            if e.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(e.path()) {
                if let Ok(card) = serde_json::from_str::<ProxmoxCard>(&content) {
                    if card.name.starts_with("proxmox") {
                        out.push(card);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}
fn ssh_cmd_output(host: &str, user: &str, remote_cmd: &str) -> Result<String, String> {
    let port = active_proxmox_default_port();
    let out = Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=5", "-p"])
        .arg(port.to_string())
        .arg(format!("{user}@{host}"))
        .arg(remote_cmd)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
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
    ssh_cmd_output(&host, &user, "echo ok")
        .map(|out| out.trim() == "ok")
        .unwrap_or(false)
}

/// 클러스터 노드 목록 (pvesh).
fn cluster_nodes() -> Vec<(String, String)> {
    let host = proxmox_host();
    let user = proxmox_user();
    let (ok, output) = common::ssh_cmd(
        &host,
        &user,
        "pvesh get /nodes --output-format json 2>/dev/null",
    );
    if !ok {
        return vec![("pve".into(), "unknown".into())];
    }
    let nodes: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap_or_default();
    nodes
        .iter()
        .map(|n| {
            let name = n
                .get("node")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let status = n
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            (name, status)
        })
        .collect()
}

/// 클러스터 전체 LXC (노드별). 반환: (node, vmid, status, name)
fn all_lxc() -> Vec<(String, String, String, String)> {
    let host = proxmox_host();
    let user = proxmox_user();
    let nodes = cluster_nodes();
    let mut result = Vec::new();
    for (node, _) in &nodes {
        let cmd = format!(
            "pvesh get /nodes/{}/lxc --output-format json 2>/dev/null",
            node
        );
        let (ok, output) = common::ssh_cmd(&host, &user, &cmd);
        if !ok {
            continue;
        }
        let ctrs: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap_or_default();
        for c in ctrs {
            let vmid = c
                .get("vmid")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default();
            let status = c
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let name = c
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
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
        let cmd = format!(
            "pvesh get /nodes/{}/qemu --output-format json 2>/dev/null",
            node
        );
        let (ok, output) = common::ssh_cmd(&host, &user, &cmd);
        if !ok {
            continue;
        }
        let vms: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap_or_default();
        for v in vms {
            let vmid = v
                .get("vmid")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default();
            let status = v
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let name = v
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            result.push((node.clone(), vmid, status, name));
        }
    }
    result.sort_by(|a, b| a.1.cmp(&b.1));
    result
}

fn lxc_lines() -> Vec<String> {
    all_lxc()
        .iter()
        .map(|(node, vmid, status, name)| {
            format!("{:<8} {:<10} {:<10} {}", vmid, status, node, name)
        })
        .collect()
}

fn lxc_name_for_vmid(vmid: &str) -> Option<String> {
    for (_, vid, _, name) in all_lxc() {
        if vid == vmid {
            return Some(name);
        }
    }
    None
}

fn normalize_lxc_key(name_or_id: &str) -> String {
    let vmid = resolve_vmid(name_or_id);
    lxc_name_for_vmid(&vmid).unwrap_or(vmid)
}

fn pct_remote_cmd(vmid: &str, inner: &str) -> String {
    let node = find_node_for_vmid(vmid);
    match node.as_deref() {
        Some("pve") | None => inner.to_string(),
        Some(remote) => format!("ssh {} '{}'", remote, inner.replace('\'', "'\\''")),
    }
}

fn pct_exec_capture(vmid: &str, inner: &str) -> Result<String, String> {
    let cmd = pct_remote_cmd(vmid, inner);
    ssh_cmd_output(&proxmox_host(), &proxmox_user(), &cmd)
}

#[derive(Debug, Clone)]
struct CurrentMountPoint {
    slot: String,
    source: String,
    target: String,
    readonly: bool,
}

fn parse_current_mount_points(vmid: &str) -> Vec<CurrentMountPoint> {
    let Ok(output) = pct_exec_capture(vmid, &format!("pct config {}", vmid)) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for line in output.lines() {
        let Some((slot, rest)) = line.split_once(':') else {
            continue;
        };
        if !slot.starts_with("mp") {
            continue;
        }
        let mut source = None;
        let mut target = None;
        let mut readonly = false;
        for (idx, part) in rest.trim().split(',').enumerate() {
            let trimmed = part.trim();
            if idx == 0 {
                source = Some(trimmed.to_string());
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("mp=") {
                target = Some(value.to_string());
            } else if trimmed == "ro=1" || trimmed == "readonly=1" {
                readonly = true;
            }
        }
        if let (Some(source), Some(target)) = (source, target) {
            result.push(CurrentMountPoint {
                slot: slot.trim().to_string(),
                source,
                target,
                readonly,
            });
        }
    }
    result
}

fn free_mount_slot(current: &[CurrentMountPoint]) -> String {
    for idx in 0..16 {
        let key = format!("mp{}", idx);
        if current.iter().all(|item| item.slot != key) {
            return key;
        }
    }
    "mp15".to_string()
}

fn mount_spec(rule: &BindMountRule) -> String {
    if rule.readonly {
        format!("{},mp={},ro=1", rule.source, rule.target)
    } else {
        format!("{},mp={}", rule.source, rule.target)
    }
}

fn host_path_exists(path: &str) -> Result<bool, String> {
    let check = format!("test -e '{}' && echo yes || echo no", path.replace('\'', "'\\''"));
    let out = ssh_cmd_output(&proxmox_host(), &proxmox_user(), &check)?;
    Ok(out.trim() == "yes")
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
    println!(
        "[등록] {} ({})",
        if proxmox_card_exists() {
            "✓ proxmox 카드"
        } else {
            "✗ env setup-proxmox 필요"
        },
        active_proxmox_card_name()
    );
    println!(
        "[Web UI] {} {}",
        proxmox_url(),
        if web_ok {
            "✓ 연결 가능"
        } else {
            "✗ 연결 불가"
        }
    );
    println!(
        "[계정] {} {}",
        user,
        if proxmox_password_exists() {
            "✓ dotenvx 비번 있음"
        } else {
            "✗ 비번 없음"
        }
    );
    println!(
        "[SSH 포트] {}:22 {}",
        host,
        if ssh_port_ok {
            "✓ 열림"
        } else {
            "✗ 닫힘"
        }
    );
    println!(
        "[SSH 로그인] {}",
        if ssh_ok {
            "✓ 키 기반 접속 가능"
        } else {
            "✗ 미설정/실패"
        }
    );
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
    let out = Command::new("open").arg(&url).output().unwrap_or_else(|e| {
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

fn cmd_bind_list() {
    let rules = load_bind_mount_rules();
    if rules.is_empty() {
        println!("선언된 bind mount가 없습니다.");
        println!("  mai run proxmox bind-add <lxc> <host-source> <container-target>");
        return;
    }
    println!(
        "{:<18} {:<38} {:<26} {}",
        "LXC", "HOST SOURCE", "TARGET", "MODE"
    );
    println!("{}", "─".repeat(100));
    for rule in rules {
        println!(
            "{:<18} {:<38} {:<26} {}",
            rule.lxc,
            rule.source,
            rule.target,
            if rule.readonly { "ro" } else { "rw" }
        );
    }
}

fn cmd_bind_add(lxc: &str, source: &str, target: &str, readonly: bool) {
    let normalized = normalize_lxc_key(lxc);
    let mut rules = load_bind_mount_rules();
    if let Some(existing) = rules
        .iter_mut()
        .find(|rule| rule.lxc == normalized && rule.target == target)
    {
        existing.source = source.to_string();
        existing.readonly = readonly;
    } else {
        rules.push(BindMountRule {
            lxc: normalized.clone(),
            source: source.to_string(),
            target: target.to_string(),
            readonly,
        });
    }
    rules.sort_by(|a, b| a.lxc.cmp(&b.lxc).then(a.target.cmp(&b.target)));
    if let Err(e) = save_bind_mount_rules(&rules) {
        eprintln!("✗ {}", e);
        std::process::exit(1);
    }
    println!(
        "✓ bind mount 선언 저장: {} {} -> {} ({})",
        normalized,
        source,
        target,
        if readonly { "ro" } else { "rw" }
    );
}

fn cmd_bind_remove(lxc: &str, target: &str) {
    let normalized = normalize_lxc_key(lxc);
    let mut rules = load_bind_mount_rules();
    let before = rules.len();
    rules.retain(|rule| !(rule.lxc == normalized && rule.target == target));
    if rules.len() == before {
        println!("일치하는 선언이 없습니다: {} {}", normalized, target);
        return;
    }
    if let Err(e) = save_bind_mount_rules(&rules) {
        eprintln!("✗ {}", e);
        std::process::exit(1);
    }
    if let Some(vmid) = all_lxc()
        .into_iter()
        .find(|(_, _, _, name)| *name == normalized)
        .map(|(_, vmid, _, _)| vmid)
    {
        let current = parse_current_mount_points(&vmid);
        if let Some(slot) = current
            .iter()
            .find(|item| item.target == target)
            .map(|item| item.slot.clone())
        {
            let cmd = pct_remote_cmd(&vmid, &format!("pct set {} -delete {}", vmid, slot));
            let (ok, out) = common::ssh_cmd(&proxmox_host(), &proxmox_user(), &cmd);
            if ok {
                println!("✓ Proxmox 설정에서도 제거: {} {}", normalized, target);
            } else {
                eprintln!("⚠ 선언은 제거했지만 pct 설정 제거는 실패: {}", out);
            }
        }
    }
    println!("✓ bind mount 선언 제거: {} {}", normalized, target);
}

fn cmd_bind_sync(target: Option<&str>) {
    let rules = load_bind_mount_rules();
    if rules.is_empty() {
        println!("동기화할 bind mount 선언이 없습니다.");
        return;
    }

    let filter = target.map(normalize_lxc_key);
    let mut matched = 0;
    let mut applied = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for (node, vmid, status, name) in all_lxc() {
        if let Some(ref wanted) = filter {
            if &name != wanted && &vmid != wanted {
                continue;
            }
        }

        let per_lxc: Vec<BindMountRule> = rules
            .iter()
            .filter(|rule| rule.lxc == name || rule.lxc == vmid)
            .cloned()
            .collect();
        if per_lxc.is_empty() {
            continue;
        }

        matched += per_lxc.len();
        if status != "running" {
            eprintln!("⚠ {}({}) stopped 상태라 bind sync는 건너뜁니다.", name, vmid);
            skipped += per_lxc.len();
            continue;
        }

        let mut current = parse_current_mount_points(&vmid);
        for rule in per_lxc {
            match host_path_exists(&rule.source) {
                Ok(true) => {}
                Ok(false) => {
                    eprintln!("✗ {}: host source 없음: {}", name, rule.source);
                    failed += 1;
                    continue;
                }
                Err(e) => {
                    eprintln!("✗ {}: host source 확인 실패: {}", name, e);
                    failed += 1;
                    continue;
                }
            }

            if current.iter().any(|item| {
                item.target == rule.target
                    && item.source == rule.source
                    && item.readonly == rule.readonly
            }) {
                skipped += 1;
                continue;
            }

            let slot = current
                .iter()
                .find(|item| item.target == rule.target)
                .map(|item| item.slot.clone())
                .unwrap_or_else(|| free_mount_slot(&current));
            let spec = mount_spec(&rule);
            let cmd = pct_remote_cmd(&vmid, &format!("pct set {} -{} {}", vmid, slot, spec));
            let (ok, out) = common::ssh_cmd(&proxmox_host(), &proxmox_user(), &cmd);
            if ok {
                println!(
                    "✓ {}({}:{}) {} -> {} [{}]",
                    name, node, vmid, rule.source, rule.target, slot
                );
                if let Some(existing) = current.iter_mut().find(|item| item.slot == slot) {
                    existing.source = rule.source.clone();
                    existing.target = rule.target.clone();
                    existing.readonly = rule.readonly;
                } else {
                    current.push(CurrentMountPoint {
                        slot: slot.clone(),
                        source: rule.source.clone(),
                        target: rule.target.clone(),
                        readonly: rule.readonly,
                    });
                }
                applied += 1;
            } else {
                eprintln!("✗ {}({}:{}) {}: {}", name, node, vmid, rule.target, out);
                failed += 1;
            }
        }
    }

    if matched == 0 {
        println!("해당 대상에 맞는 bind mount 선언이 없습니다.");
        return;
    }
    println!(
        "\nbind-sync: 적용 {}, 유지 {}, 실패 {}",
        applied, skipped, failed
    );
}

fn vm_lines() -> Vec<String> {
    all_vms()
        .iter()
        .map(|(node, vmid, status, name)| {
            format!("{:<8} {:<10} {:<10} {}", vmid, status, node, name)
        })
        .collect()
}

fn cmd_vm_list() {
    if !ssh_login_ok() {
        eprintln!("✗ SSH 접속 불가");
        std::process::exit(1);
    }
    let lines = vm_lines();
    if lines.is_empty() {
        println!("VM 없음");
        return;
    }
    println!("=== Proxmox VM ===\n");
    for line in lines {
        println!("  {}", line);
    }
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
        if vid == vmid {
            return Some(node);
        }
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
    println!(
        "LXC {} 셸 접속 중... ({})",
        vmid,
        node.as_deref().unwrap_or("local")
    );
    let _ = Command::new("ssh").args(["-t", &target, &cmd]).status();
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
        Err(e) => {
            eprintln!("✗ {}", e);
            std::process::exit(1);
        }
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
    if ok {
        println!(
            "✓ LXC {} 시작 ({})",
            vmid,
            node.as_deref().unwrap_or("local")
        );
    } else {
        eprintln!("✗ {}", out);
    }
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
    if ok {
        println!(
            "✓ LXC {} 정지 ({})",
            vmid,
            node.as_deref().unwrap_or("local")
        );
    } else {
        eprintln!("✗ {}", out);
    }
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
    let ssh_ok = ssh_login_ok();

    let usage_active = proxmox_card_exists();
    let usage_summary = if usage_active {
        format!(
            "{} / web {}",
            proxmox_url(),
            if web_ok { "ok" } else { "down" }
        )
    } else {
        "미등록".to_string()
    };

    // 클러스터 전체 데이터
    let lxc_all = if ssh_ok { all_lxc() } else { Vec::new() };
    let vm_all = if ssh_ok { all_vms() } else { Vec::new() };
    let nodes = if ssh_ok { cluster_nodes() } else { Vec::new() };

    let lxc_rows: Vec<serde_json::Value> = lxc_all
        .iter()
        .map(|(node, vmid, status, name)| serde_json::json!([vmid, status, node, name]))
        .collect();

    let vm_rows: Vec<serde_json::Value> = vm_all
        .iter()
        .map(|(node, vmid, status, name)| serde_json::json!([vmid, status, node, name]))
        .collect();

    let lxc_running = lxc_all.iter().filter(|c| c.2 == "running").count();
    let lxc_total = lxc_all.len();
    let node_info = nodes
        .iter()
        .map(|(n, s)| format!("{} ({})", n, s))
        .collect::<Vec<_>>()
        .join(", ");

    TuiSpec::new("proxmox")
        .refresh(30)
        .usage(usage_active, &usage_summary)
        .kv(
            "상태",
            vec![
                tui_spec::kv_item(
                    "클러스터",
                    &node_info,
                    if !nodes.is_empty() { "ok" } else { "error" },
                ),
                tui_spec::kv_item(
                    "Web UI",
                    &proxmox_url(),
                    if web_ok { "ok" } else { "error" },
                ),
                tui_spec::kv_item(
                    "SSH",
                    &format!("{}@{}:{}", user, host, active_proxmox_default_port()),
                    if ssh_ok { "ok" } else { "warn" },
                ),
                tui_spec::kv_item(
                    "LXC",
                    &format!("{}/{} running", lxc_running, lxc_total),
                    if lxc_running > 0 { "ok" } else { "warn" },
                ),
                tui_spec::kv_item("VM", &format!("{}", vm_all.len()), "ok"),
            ],
        )
        .table(
            "LXC 컨테이너",
            vec!["VMID", "STATUS", "NODE", "NAME"],
            lxc_rows,
        )
        .table("VM", vec!["VMID", "STATUS", "NODE", "NAME"], vm_rows)
        .buttons()
        .text(
            "안내",
            "  mai run proxmox ssh              호스트 SSH 접속\n  \
             mai run proxmox lxc-shell 50063   LXC 셸 접속\n  \
             mai run proxmox lxc-exec 50063 ls 명령 실행\n  \
             mai run proxmox lxc-start 50063   LXC 시작\n  \
             mai run proxmox lxc-stop 50063    LXC 정지",
        )
        .print();
}
