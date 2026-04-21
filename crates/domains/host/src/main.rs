use clap::{Parser, Subcommand};
use mac_common::cmd;
use std::fs;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-host")]
#[command(about = "macOS 호스트 상태 + /etc/hosts 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 호스트 상태 요약
    Status,
    /// /etc/hosts 전체 읽기
    Hosts,
    /// /etc/hosts 에 항목 추가 안내 (sudo 명령 출력)
    Add { ip: String, hostname: String },
    /// 호스트명/IP 검색
    Search { query: String },
    /// DNS 관리
    Dns {
        #[command(subcommand)]
        action: DnsAction,
    },
    /// TUI 스펙 (JSON)
    TuiSpec,
}

#[derive(Subcommand)]
enum DnsAction {
    /// 인터페이스별 DNS 현황
    Status,
    /// DNS 서버 설정 (프리셋 이름 또는 IP)
    Set {
        /// 네트워크 인터페이스 (Wi-Fi, Ethernet 등)
        interface: String,
        /// 프리셋(google/cloudflare/quad9/adguard/opendns) 또는 IP 주소
        value: String,
        /// 보조 DNS (IP 직접 입력 시)
        #[arg(long)]
        secondary: Option<String>,
    },
    /// DNS 서버 초기화 (DHCP 기본)
    Reset {
        /// 네트워크 인터페이스
        interface: String,
    },
    /// DNS 캐시 플러시
    Flush,
    /// DNS 조회 테스트
    Test {
        /// 조회할 도메인
        domain: String,
        /// 사용할 DNS 서버 (생략 시 시스템 기본)
        #[arg(long, short)]
        server: Option<String>,
    },
    /// DNS 프리셋 목록
    Presets,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Hosts => cmd_hosts(),
        Commands::Add { ip, hostname } => cmd_add(&ip, &hostname),
        Commands::Search { query } => cmd_search(&query),
        Commands::Dns { action } => match action {
            DnsAction::Status => dns_status(),
            DnsAction::Set { interface, value, secondary } => dns_set(&interface, &value, secondary.as_deref()),
            DnsAction::Reset { interface } => dns_reset(&interface),
            DnsAction::Flush => dns_flush(),
            DnsAction::Test { domain, server } => dns_test(&domain, server.as_deref()),
            DnsAction::Presets => dns_presets(),
        },
        Commands::TuiSpec => print_tui_spec(),
    }
}

const HOSTS_PATH: &str = "/etc/hosts";

#[derive(Debug)]
struct HostEntry {
    ip: String,
    hostname: String,
    comment: bool,
}

fn parse_hosts() -> Vec<HostEntry> {
    let content = fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("##") || trimmed.starts_with("# =") {
            continue;
        }
        let (comment, effective) = if let Some(stripped) = trimmed.strip_prefix('#') {
            (true, stripped.trim())
        } else { (false, trimmed) };
        let parts: Vec<&str> = effective.split_whitespace().collect();
        if parts.len() >= 2 {
            entries.push(HostEntry {
                ip: parts[0].into(),
                hostname: parts[1..].join(" "),
                comment,
            });
        }
    }
    entries
}

fn uptime() -> String {
    Command::new("uptime").output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn disk_usage_root() -> String {
    Command::new("df").args(["-h", "/"]).output().ok()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.lines().nth(1).map(|l| {
                let p: Vec<&str> = l.split_whitespace().collect();
                if p.len() >= 5 {
                    format!("{} used / {} total ({})", p[2], p[1], p[4])
                } else { l.to_string() }
            }).unwrap_or_default()
        })
        .unwrap_or_default()
}

fn memory_summary() -> String {
    let out = Command::new("vm_stat").output().ok();
    let Some(o) = out else { return String::new(); };
    let text = String::from_utf8_lossy(&o.stdout);
    let mut free = 0u64; let mut active = 0u64; let mut wired = 0u64; let mut compressed = 0u64;
    let parse = |s: &str| s.trim().trim_end_matches('.').parse::<u64>().ok();
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("Pages free:").and_then(parse) { free = v; }
        else if let Some(v) = line.strip_prefix("Pages active:").and_then(parse) { active = v; }
        else if let Some(v) = line.strip_prefix("Pages wired down:").and_then(parse) { wired = v; }
        else if let Some(v) = line.strip_prefix("Pages occupied by compressor:").and_then(parse) { compressed = v; }
    }
    // Apple Silicon 페이지 크기 16KB.
    let page = 16384u64;
    let mb = |p: u64| (p * page) / 1_000_000;
    format!("free {}MB, active {}MB, wired {}MB, compressed {}MB",
        mb(free), mb(active), mb(wired), mb(compressed))
}

fn cmd_status() {
    println!("=== Host Status ===\n");
    println!("uptime : {}", uptime());
    println!("disk / : {}", disk_usage_root());
    println!("memory : {}", memory_summary());
    let entries = parse_hosts();
    let active = entries.iter().filter(|e| !e.comment).count();
    let commented = entries.iter().filter(|e| e.comment).count();
    println!("/etc/hosts: {} 활성, {} 주석", active, commented);
}

fn cmd_hosts() {
    let entries = parse_hosts();
    println!("{:<18} {:<30} {}", "IP", "HOSTNAME", "STATE");
    println!("{}", "─".repeat(60));
    for e in entries {
        let state = if e.comment { "주석" } else { "활성" };
        println!("{:<18} {:<30} {}", e.ip, e.hostname, state);
    }
}

fn cmd_add(ip: &str, hostname: &str) {
    let line = format!("{}\t{}", ip, hostname);
    println!("추가할 줄: {}", line);
    println!("sudo 권한 필요. 다음 명령을 직접 실행하세요:");
    println!("  echo '{}' | sudo tee -a /etc/hosts > /dev/null", line);
}

fn cmd_search(query: &str) {
    let q = query.to_lowercase();
    let entries = parse_hosts();
    let mut hits = 0;
    for e in entries {
        if e.ip.to_lowercase().contains(&q) || e.hostname.to_lowercase().contains(&q) {
            let state = if e.comment { "#" } else { " " };
            println!("{} {:<18} {}", state, e.ip, e.hostname);
            hits += 1;
        }
    }
    if hits == 0 { println!("매칭 없음"); }
}

fn print_tui_spec() {
    use mac_common::tui_spec::{self, TuiSpec};

    let entries = parse_hosts();
    let host_items: Vec<serde_json::Value> = entries.iter().map(|e| {
        tui_spec::kv_item_data(&e.ip, &e.hostname,
            if e.comment { "warn" } else { "ok" },
            serde_json::json!({ "ip": e.ip, "hostname": e.hostname,
                      "commented": e.comment.to_string() }))
    }).collect();

    TuiSpec::new("host")
        .refresh(30)
        .list_section("/etc/hosts")
        .usage(true, "항상 활성")
        .kv("상태", vec![
            tui_spec::kv_item("uptime", &uptime(), "ok"),
            tui_spec::kv_item("disk /", &disk_usage_root(), "ok"),
            tui_spec::kv_item("memory", &memory_summary(), "ok"),
        ])
        .kv("/etc/hosts", host_items)
        .kv("DNS", dns_status_items())
        .buttons()
        .text("안내", "편집은 sudo 필요. CLI:\n  mai run host add <ip> <hostname>\n  mai run host dns set Wi-Fi cloudflare\n  mai run host dns flush")
        .print();
}

// ═══════════════════════════════════════
// DNS 관리
// ═══════════════════════════════════════

/// DNS 프리셋 조회. locale.json의 dns_presets에서 읽음.
fn dns_preset(name: &str) -> Option<(String, String)> {
    mac_common::locale::dns_preset(name).map(|p| (p.primary, p.secondary))
}

/// networksetup으로 인터페이스 DNS 조회.
fn get_dns(interface: &str) -> String {
    cmd::stdout("networksetup", &["-getdnsservers", interface])
}

/// 활성 네트워크 인터페이스 목록.
fn list_interfaces() -> Vec<String> {
    let out = cmd::stdout("networksetup", &["-listallnetworkservices"]);
    out.lines()
        .skip(1) // 첫 줄: "An asterisk (*) denotes ..."
        .map(|l| l.trim_start_matches('*').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// TUI 상태 섹션용 DNS 아이템.
fn dns_status_items() -> Vec<serde_json::Value> {
    use mac_common::tui_spec;
    let interfaces = list_interfaces();
    let mut items = Vec::new();
    for iface in &interfaces {
        let dns = get_dns(iface);
        let (value, status) = if dns.contains("aren't any") || dns.contains("no DNS") {
            ("DHCP (자동)".to_string(), "warn")
        } else {
            let servers: Vec<&str> = dns.lines().collect();
            (servers.join(", "), "ok")
        };
        items.push(tui_spec::kv_item_data(
            &format!("DNS ({})", iface), &value, status,
            serde_json::json!({ "name": iface, "interface": iface }),
        ));
    }
    items
}

fn dns_status() {
    println!("=== DNS 현황 ===\n");

    let interfaces = list_interfaces();
    for iface in &interfaces {
        let dns = get_dns(iface);
        if dns.contains("aren't any") || dns.contains("no DNS") {
            println!("  {:<20} DHCP (자동)", iface);
        } else {
            let servers: Vec<&str> = dns.lines().collect();
            println!("  {:<20} {}", iface, servers.join(", "));
        }
    }

    // scutil 전체 resolver 체인
    println!("\n=== Resolver 체인 (scutil --dns) ===\n");
    let scutil = cmd::stdout("scutil", &["--dns"]);
    // resolver 요약만
    for line in scutil.lines() {
        if line.contains("nameserver") || line.contains("domain") || line.contains("search") || line.starts_with("resolver") {
            println!("  {}", line.trim());
        }
    }
}

fn dns_set(interface: &str, value: &str, secondary: Option<&str>) {
    // 프리셋 확인
    let (primary, sec) = if let Some((p, s)) = dns_preset(value) {
        (p.to_string(), s.to_string())
    } else if secondary.is_some() {
        (value.to_string(), secondary.unwrap().to_string())
    } else {
        // IP 하나만 입력
        (value.to_string(), String::new())
    };

    let mut args = vec!["-setdnsservers", interface, &primary];
    if !sec.is_empty() {
        args.push(&sec);
    }

    println!("DNS 설정: {} → {} {}", interface, primary, sec);
    println!("  (sudo 필요)");
    println!();

    let status = Command::new("sudo")
        .args(["networksetup"])
        .args(&args)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✓ DNS 설정 완료");
            println!("  적용 확인: mai run host dns status");
            // 자동 플러시
            let _ = Command::new("sudo").args(["dscacheutil", "-flushcache"]).status();
            let _ = Command::new("sudo").args(["killall", "-HUP", "mDNSResponder"]).status();
            println!("✓ DNS 캐시 플러시 완료");
        }
        _ => eprintln!("✗ DNS 설정 실패 (sudo 권한 확인)"),
    }
}

fn dns_reset(interface: &str) {
    println!("DNS 초기화: {} → DHCP 기본", interface);
    let status = Command::new("sudo")
        .args(["networksetup", "-setdnsservers", interface, "Empty"])
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("✓ DNS 초기화 완료");
            let _ = Command::new("sudo").args(["dscacheutil", "-flushcache"]).status();
            let _ = Command::new("sudo").args(["killall", "-HUP", "mDNSResponder"]).status();
            println!("✓ DNS 캐시 플러시 완료");
        }
        _ => eprintln!("✗ DNS 초기화 실패"),
    }
}

fn dns_flush() {
    println!("DNS 캐시 플러시 중...");
    let r1 = Command::new("sudo").args(["dscacheutil", "-flushcache"]).status();
    let r2 = Command::new("sudo").args(["killall", "-HUP", "mDNSResponder"]).status();
    match (r1, r2) {
        (Ok(s1), Ok(s2)) if s1.success() && s2.success() => {
            println!("✓ DNS 캐시 플러시 완료");
        }
        _ => eprintln!("✗ 플러시 실패 (sudo 권한 확인)"),
    }
}

fn dns_test(domain: &str, server: Option<&str>) {
    println!("=== DNS 조회: {} ===\n", domain);

    // nslookup
    let args = if let Some(srv) = server {
        println!("  서버: {}\n", srv);
        vec![domain, srv]
    } else {
        vec![domain]
    };
    let out = cmd::output("nslookup", &args.iter().map(|s| s.as_ref()).collect::<Vec<&str>>());
    for line in out.lines() {
        println!("  {}", line);
    }
}

fn dns_presets() {
    let presets = mac_common::locale::dns_presets();
    if presets.is_empty() {
        println!("프리셋 없음 (locale.json 확인: nickel export ncl/domains.ncl > ~/.mac-app-init/locale.json)");
        return;
    }
    println!("=== DNS 프리셋 ===\n");
    println!("  {:<20} {:<18} {:<18} {}", "이름", "Primary", "Secondary", "설명");
    println!("  {}", "─".repeat(75));
    let mut names: Vec<&String> = presets.keys().collect();
    names.sort();
    for name in names {
        let p = &presets[name];
        println!("  {:<20} {:<18} {:<18} {}", name, p.primary, p.secondary, p.description);
    }
    println!("\n  사용법: mai run host dns set Wi-Fi cloudflare");
    println!("  초기화: mai run host dns reset Wi-Fi");
}
