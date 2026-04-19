use clap::{Parser, Subcommand};
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
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Hosts => cmd_hosts(),
        Commands::Add { ip, hostname } => cmd_add(&ip, &hostname),
        Commands::Search { query } => cmd_search(&query),
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
    let entries = parse_hosts();
    let host_items: Vec<serde_json::Value> = entries.iter().map(|e| {
        serde_json::json!({
            "key": e.ip,
            "value": e.hostname,
            "status": if e.comment { "warn" } else { "ok" },
            "data": { "ip": e.ip, "hostname": e.hostname,
                      "commented": e.comment.to_string() }
        })
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label_ko": "시스템 상태", "label": "Host", "icon": "🖥" },
        "group": "infra",        "list_section": "/etc/hosts",
        "sections": [
            {
                "kind": "key-value",
                "title": "System",
                "items": [
                    { "key": "uptime", "value": uptime(), "status": "ok" },
                    { "key": "disk /", "value": disk_usage_root(), "status": "ok" },
                    { "key": "memory", "value": memory_summary(), "status": "ok" }
                ]
            },
            {
                "kind": "key-value",
                "title": "/etc/hosts",
                "items": host_items
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status", "command": "status", "key": "s" },
                    { "label": "All hosts", "command": "hosts", "key": "l" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "편집은 sudo 필요. CLI:\n  mac run host add <ip> <hostname>\n  sudo nano /etc/hosts"
            }
        ],
        "keybindings": []
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
