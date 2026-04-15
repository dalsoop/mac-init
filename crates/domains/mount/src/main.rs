use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-mount")]
#[command(about = "SMB/NFS 공유 마운트 관리 (connect 필요)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 (connections + 현재 마운트)
    Status,
    /// 사용 가능한 공유 스캔 (connections.json 기준)
    Shares,
    /// 현재 SMB/NFS 마운트 목록
    List,
    /// 공유 마운트
    Mount {
        /// 연결 이름 (connect 도메인에 등록된 것)
        name: String,
        /// 공유 이름
        share: String,
    },
    /// 공유 언마운트
    Unmount {
        /// 연결 이름 또는 마운트 경로
        target: String,
    },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

fn connections_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/connections.json")
}

fn env_path() -> PathBuf {
    PathBuf::from(home()).join(".env")
}

fn has_connect_domain() -> bool {
    PathBuf::from(home()).join(".mac-app-init/domains/mac-domain-connect").exists()
}

#[derive(Debug)]
struct Connection {
    name: String,
    host: String,
    user: String,
    #[allow(dead_code)]
    port: u16,
}

fn load_connections() -> Vec<Connection> {
    let path = connections_path();
    if !path.exists() { return Vec::new(); }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    json.get("services").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| {
            Some(Connection {
                name: s.get("name")?.as_str()?.to_string(),
                host: s.get("host")?.as_str()?.to_string(),
                user: s.get("user")?.as_str()?.to_string(),
                port: s.get("port")?.as_u64()? as u16,
            })
        }).collect())
        .unwrap_or_default()
}

fn find_connection(name: &str) -> Option<Connection> {
    load_connections().into_iter().find(|c| c.name == name)
}

fn dotenvx_get(key: &str) -> Option<String> {
    let out = Command::new("dotenvx")
        .args(["get", key, "-f", &env_path().to_string_lossy()])
        .output().ok()?;
    if !out.status.success() { return None; }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() { None } else { Some(v) }
}

fn get_password(name: &str) -> Option<String> {
    let key = format!("{}_PASSWORD", name.to_uppercase().replace('-', "_"));
    dotenvx_get(&key)
}

fn url_encode(s: &str) -> String {
    s.chars().flat_map(|c| {
        match c {
            '@' => "%40".chars().collect::<Vec<_>>(),
            '#' => "%23".chars().collect::<Vec<_>>(),
            ':' => "%3A".chars().collect::<Vec<_>>(),
            '/' => "%2F".chars().collect::<Vec<_>>(),
            '?' => "%3F".chars().collect::<Vec<_>>(),
            ' ' => "%20".chars().collect::<Vec<_>>(),
            c => vec![c],
        }
    }).collect()
}

fn list_smb_shares(conn: &Connection, password: &str) -> Vec<String> {
    let url = format!("//{}:{}@{}", conn.user, url_encode(password), conn.host);
    let out = Command::new("smbutil").args(["view", &url]).output();
    let Ok(o) = out else { return Vec::new(); };
    if !o.status.success() { return Vec::new(); }
    let stdout = String::from_utf8_lossy(&o.stdout);
    stdout.lines()
        .skip(2)
        .filter_map(|l| {
            let first = l.split_whitespace().next()?;
            if first.ends_with('$') || first.starts_with('-') || first.is_empty() { return None; }
            Some(first.to_string())
        })
        .filter(|s| !s.contains("shares") && !s.contains("listed"))
        .collect()
}

fn list_current_mounts() -> Vec<(String, String)> {
    let out = Command::new("mount").output();
    let Ok(o) = out else { return Vec::new(); };
    String::from_utf8_lossy(&o.stdout).lines()
        .filter(|l| l.contains("smbfs") || l.contains("nfs") || l.contains("macfuse"))
        .filter_map(|l| {
            let parts: Vec<&str> = l.splitn(4, ' ').collect();
            if parts.len() >= 3 && parts[1] == "on" {
                Some((parts[0].to_string(), parts[2].to_string()))
            } else { None }
        })
        .collect()
}

fn main() {
    if !has_connect_domain() {
        eprintln!("⚠ connect 도메인이 필요합니다: mac install connect");
    }
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Shares => cmd_shares(),
        Commands::List => cmd_list(),
        Commands::Mount { name, share } => cmd_mount(&name, &share),
        Commands::Unmount { target } => cmd_unmount(&target),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn cmd_status() {
    let conns = load_connections();
    let mounts = list_current_mounts();
    println!("=== Mount Status ===\n");
    println!("연결 ({}개):", conns.len());
    for c in &conns {
        println!("  • {:<12} {}@{}", c.name, c.user, c.host);
    }
    println!("\n마운트 ({}개):", mounts.len());
    for (src, mp) in &mounts {
        println!("  ✓ {} → {}", src, mp);
    }
}

fn cmd_shares() {
    let conns = load_connections();
    if conns.is_empty() {
        println!("등록된 연결이 없습니다. mac run connect add <name>");
        return;
    }
    for c in &conns {
        println!("=== {} ({}) ===", c.name, c.host);
        let Some(pw) = get_password(&c.name) else {
            println!("  (비번 없음 — .env 에 {}_PASSWORD 필요)\n", c.name.to_uppercase());
            continue;
        };
        let shares = list_smb_shares(c, &pw);
        if shares.is_empty() {
            println!("  (공유 없음 또는 접근 불가)");
        } else {
            for s in shares {
                println!("  • {}", s);
            }
        }
        println!();
    }
}

fn cmd_list() {
    let mounts = list_current_mounts();
    if mounts.is_empty() {
        println!("현재 마운트된 SMB/NFS 공유가 없습니다.");
        return;
    }
    println!("{:<40} {}", "SOURCE", "MOUNTPOINT");
    println!("{}", "─".repeat(80));
    for (src, mp) in mounts {
        println!("{:<40} {}", src, mp);
    }
}

fn cmd_mount(name: &str, share: &str) {
    let Some(conn) = find_connection(name) else {
        eprintln!("✗ 연결 '{}' 이(가) 없습니다. mac run connect list", name);
        return;
    };
    let Some(pw) = get_password(name) else {
        eprintln!("✗ {}_PASSWORD 가 .env 에 없습니다.", name.to_uppercase());
        return;
    };
    let url = format!(
        "smb://{}:{}@{}/{}",
        url_encode(&conn.user),
        url_encode(&pw),
        conn.host,
        url_encode(share),
    );
    println!("마운트 중: {}/{}", conn.host, share);
    let out = Command::new("open").arg(&url).output();
    match out {
        Ok(o) if o.status.success() => {
            println!("✓ /Volumes/{} (또는 유사 경로) 로 마운트됨", share);
        }
        Ok(o) => {
            eprintln!("✗ 마운트 실패: {}", String::from_utf8_lossy(&o.stderr));
        }
        Err(e) => eprintln!("✗ open 실행 실패: {}", e),
    }
}

fn cmd_unmount(target: &str) {
    let path = if target.starts_with('/') {
        target.to_string()
    } else {
        format!("/Volumes/{}", target)
    };
    let out = Command::new("diskutil").args(["unmount", &path]).output();
    match out {
        Ok(o) if o.status.success() => println!("{}", String::from_utf8_lossy(&o.stdout).trim()),
        Ok(o) => eprintln!("{}", String::from_utf8_lossy(&o.stderr).trim()),
        Err(e) => eprintln!("✗ diskutil 실패: {}", e),
    }
}

fn print_tui_spec() {
    let conns = load_connections();
    let mounts = list_current_mounts();

    let mount_rows: Vec<serde_json::Value> = mounts.iter()
        .map(|(src, mp)| serde_json::json!([src, mp]))
        .collect();

    let mut pw_map: HashMap<String, bool> = HashMap::new();
    for c in &conns {
        pw_map.insert(c.name.clone(), get_password(&c.name).is_some());
    }
    let conn_rows: Vec<serde_json::Value> = conns.iter().map(|c| {
        let has_pw = pw_map.get(&c.name).copied().unwrap_or(false);
        serde_json::json!([
            c.name,
            format!("{}@{}", c.user, c.host),
            if has_pw { "✓" } else { "✗" },
        ])
    }).collect();

    let connect_ok = has_connect_domain();

    let spec = serde_json::json!({
        "tab": { "label": "Mount", "icon": "💾" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    {
                        "key": "connect 도메인",
                        "value": if connect_ok { "✓ 설치됨" } else { "✗ 미설치 (mac install connect)" },
                        "status": if connect_ok { "ok" } else { "error" }
                    },
                    {
                        "key": "등록된 연결",
                        "value": format!("{}개", conns.len()),
                        "status": if conns.is_empty() { "warn" } else { "ok" }
                    },
                    {
                        "key": "현재 마운트",
                        "value": format!("{}개", mounts.len()),
                        "status": "ok"
                    }
                ]
            },
            {
                "kind": "table",
                "title": "연결 (비번)",
                "headers": ["NAME", "ENDPOINT", "PW"],
                "rows": conn_rows
            },
            {
                "kind": "table",
                "title": "마운트된 공유",
                "headers": ["SOURCE", "MOUNTPOINT"],
                "rows": mount_rows
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status", "command": "status", "key": "s" },
                    { "label": "Shares (공유 스캔)", "command": "shares", "key": "h" },
                    { "label": "List (현재 마운트)", "command": "list", "key": "l" }
                ]
            },
            {
                "kind": "text",
                "title": "마운트 / 언마운트 — 터미널",
                "content": "  마운트:   mac run mount mount <name> <share>\n  언마운트: mac run mount unmount <share>\n\n  예시:\n    mac run mount shares                    # 모든 연결에서 공유 스캔\n    mac run mount mount synology 컨텐츠\n    mac run mount unmount 컨텐츠"
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
