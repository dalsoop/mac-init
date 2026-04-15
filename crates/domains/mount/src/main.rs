use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod backend;
#[cfg(all(target_os = "macos", feature = "netfs"))]
mod netfs;

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
    /// 자동 마운트 설정 추가
    AutoAdd {
        /// 연결 이름
        connection: String,
        /// 공유 이름
        share: String,
    },
    /// 자동 마운트 설정 제거
    AutoRemove {
        connection: String,
        share: String,
    },
    /// 자동 마운트 설정 토글
    AutoToggle {
        connection: String,
        share: String,
    },
    /// 자동 마운트 설정 목록
    AutoList,
    /// 자동 마운트 실행 (config 의 enabled 항목들 중 미마운트인 것 마운트)
    Auto,
    /// LaunchAgent 등록 (로그인 시 + 5분마다 mount auto 실행)
    AutoEnable,
    /// LaunchAgent 제거
    AutoDisable,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

/// 통합 마운트 루트 (~/NAS)
fn nas_root() -> PathBuf {
    PathBuf::from(home()).join("NAS")
}

/// 마운트 포인트: ~/NAS/<conn>/<share>
fn mount_point(connection: &str, share: &str) -> PathBuf {
    nas_root().join(connection).join(share)
}

/// mount_smbfs 호출. ASCII share 는 직접, 비-ASCII (한글 등) 는 open smb:// fallback.
fn mount_smbfs(user: &str, password: &str, host: &str, share: &str, mp: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(mp).map_err(|e| format!("디렉터리 생성 실패: {}", e))?;

    if share.is_ascii() {
        let url = format!(
            "//{}:{}@{}/{}",
            url_encode(user),
            url_encode(password),
            host,
            url_encode(share),
        );
        let out = Command::new("mount_smbfs")
            .args(["-o", "soft,nobrowse"])
            .arg(&url)
            .arg(mp)
            .output()
            .map_err(|e| format!("mount_smbfs 실행 실패: {}", e))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    } else {
        // 한글 등 비-ASCII share — mount_smbfs 가 처리 못함, open 사용 (/Volumes/<share> 로 마운트됨)
        let url = format!(
            "smb://{}:{}@{}/{}",
            url_encode(user),
            url_encode(password),
            host,
            url_encode(share),
        );
        let out = Command::new("open").arg(&url).output()
            .map_err(|e| format!("open 실행 실패: {}", e))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        // open 은 비동기. /Volumes/<share> 에 마운트되길 잠깐 대기.
        let vol_path = PathBuf::from(format!("/Volumes/{}", share));
        for _ in 0..15 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if vol_path.exists() {
                // ~/NAS/<conn>/<share> 가 비어있으면 심볼릭 링크로 연결
                let _ = fs::remove_dir(mp); // 빈 디렉터리만 지워짐
                if !mp.exists() {
                    let _ = std::os::unix::fs::symlink(&vol_path, mp);
                }
                return Ok(());
            }
        }
        Err(format!("open smb 마운트 타임아웃 ({})", vol_path.display()))
    }
}

/// 경로가 stale 한지 확인 — 백그라운드 ls + 2초 wait, hang 이면 stale 로 간주.
fn is_stale(mp: &PathBuf) -> bool {
    use std::sync::mpsc;
    use std::thread;
    let path = mp.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let ok = fs::read_dir(&path).map(|d| d.count()).is_ok();
        let _ = tx.send(ok);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(true) => false,
        Ok(false) => true,
        Err(_) => true, // timeout
    }
}

fn unmount_path(mp: &PathBuf) -> Result<(), String> {
    let out = Command::new("diskutil").args(["unmount", "force"]).arg(mp).output()
        .or_else(|_| Command::new("umount").arg("-f").arg(mp).output())
        .map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) }
    else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
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
    let result: Vec<Connection> = json.get("services").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| {
            Some(Connection {
                name: s.get("name")?.as_str()?.to_string(),
                host: s.get("host")?.as_str()?.to_string(),
                user: s.get("user")?.as_str()?.to_string(),
                port: s.get("port")?.as_u64()? as u16,
            })
        }).collect())
        .unwrap_or_default();
    if !result.is_empty() {
        eprintln!(
            "⚠ legacy {} 를 읽는 중. `mac run env import` 로 카드로 이관 후 파일을 삭제하세요.",
            path.display()
        );
    }
    result
}

fn find_connection(name: &str) -> Option<Connection> {
    // 1순위: env 카드. 2순위: legacy connections.json
    if let Some(c) = env_card_show(name) {
        return Some(c);
    }
    load_connections().into_iter().find(|c| c.name == name)
}

/// env 카드 전체 목록 + legacy connections.json 를 합친 결과 (카드 우선, 이름 중복 제거).
fn load_all_connections() -> Vec<Connection> {
    let out = Command::new(env_binary()).args(["list"]).output();
    let mut cards: Vec<Connection> = Vec::new();
    if let Ok(o) = out {
        if o.status.success() {
            // parse 는 `env list` 테이블 대신 카드 파일 직접 읽기가 간결
            let dir = PathBuf::from(home()).join(".mac-app-init/cards");
            if let Ok(it) = std::fs::read_dir(&dir) {
                for e in it.filter_map(|x| x.ok()) {
                    if e.path().extension().and_then(|s| s.to_str()) != Some("json") { continue; }
                    if let Ok(content) = std::fs::read_to_string(e.path()) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let (Some(name), Some(host), Some(user), Some(port)) = (
                                v.get("name").and_then(|x| x.as_str()),
                                v.get("host").and_then(|x| x.as_str()),
                                v.get("user").and_then(|x| x.as_str()),
                                v.get("port").and_then(|x| x.as_u64()),
                            ) {
                                cards.push(Connection {
                                    name: name.into(), host: host.into(), user: user.into(), port: port as u16,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    let card_names: std::collections::HashSet<String> =
        cards.iter().map(|c| c.name.clone()).collect();
    for c in load_connections() {
        if !card_names.contains(&c.name) {
            cards.push(c);
        }
    }
    cards.sort_by(|a, b| a.name.cmp(&b.name));
    cards
}

/// env 도메인 바이너리 경로.
/// 1) PATH 2) ~/.mac-app-init/domains/mac-domain-env 3) ./target/debug/mac-domain-env
fn env_binary() -> PathBuf {
    let candidates = [
        PathBuf::from("mac-domain-env"),
        PathBuf::from(home()).join(".mac-app-init/domains/mac-domain-env"),
        PathBuf::from("./target/debug/mac-domain-env"),
        PathBuf::from("./target/release/mac-domain-env"),
    ];
    for c in &candidates[1..] {
        if c.exists() { return c.clone(); }
    }
    candidates[0].clone()
}

/// env 도메인 CLI 로 카드 조회 → Connection 으로 변환.
fn env_card_show(name: &str) -> Option<Connection> {
    let out = Command::new(env_binary())
        .args(["show", name])
        .output().ok()?;
    if !out.status.success() { return None; }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(Connection {
        name: json.get("name")?.as_str()?.to_string(),
        host: json.get("host")?.as_str()?.to_string(),
        user: json.get("user")?.as_str()?.to_string(),
        port: json.get("port")?.as_u64()? as u16,
    })
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
    // 1순위: env 카드 (keychain). 2순위: legacy .env
    if let Some(pw) = env_card_password(name) {
        return Some(pw);
    }
    let key = format!("{}_PASSWORD", name.to_uppercase().replace('-', "_"));
    dotenvx_get(&key)
}

fn env_card_password(name: &str) -> Option<String> {
    let out = Command::new(env_binary())
        .args(["get-password", name])
        .output().ok()?;
    if !out.status.success() { return None; }
    let v = String::from_utf8_lossy(&out.stdout).to_string();
    if v.is_empty() { None } else { Some(v) }
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

/// 마운트 포인트가 실제로 활성 상태인지 (symlink 대상 포함)
fn is_mounted_at(mp: &PathBuf) -> bool {
    if !mp.exists() { return false; }
    let active: Vec<String> = list_current_mounts().into_iter().map(|(_, m)| m).collect();
    let mp_str = mp.to_string_lossy().to_string();
    if active.iter().any(|m| m == &mp_str) { return true; }
    // symlink 라면 target 확인
    if let Ok(target) = fs::read_link(mp) {
        let target_str = target.to_string_lossy().to_string();
        return active.iter().any(|m| m == &target_str);
    }
    false
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
        Commands::AutoAdd { connection, share } => cmd_auto_add(&connection, &share),
        Commands::AutoRemove { connection, share } => cmd_auto_remove(&connection, &share),
        Commands::AutoToggle { connection, share } => cmd_auto_toggle(&connection, &share),
        Commands::AutoList => cmd_auto_list(),
        Commands::Auto => cmd_auto(),
        Commands::AutoEnable => cmd_auto_enable(),
        Commands::AutoDisable => cmd_auto_disable(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn cmd_status() {
    let conns = load_all_connections();
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
    let conns = load_all_connections();
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
    let mp = mount_point(name, share);
    println!("마운트 중: {} → {}", conn.host, mp.display());
    let req = backend::MountRequest {
        host: &conn.host,
        share,
        user: &conn.user,
        password: &pw,
        mountpoint: &mp,
    };
    let result = backend::mount(&req, |r| {
        mount_smbfs(r.user, r.password, r.host, r.share, &r.mountpoint.to_path_buf())
    });
    match result {
        Ok(backend_name) => println!("✓ {} ({})", mp.display(), backend_name),
        Err(e) => eprintln!("✗ {}", e),
    }
}

// === Auto-mount config ===

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutoMount {
    connection: String,
    share: String,
    #[serde(default = "default_true")]
    enabled: bool,
}
fn default_true() -> bool { true }

#[derive(Debug, Default, Serialize, Deserialize)]
struct MountConfig {
    #[serde(default)]
    auto_mounts: Vec<AutoMount>,
}

fn mount_config_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/mount.json")
}

fn load_mount_config() -> MountConfig {
    let path = mount_config_path();
    if !path.exists() { return MountConfig::default(); }
    serde_json::from_str(&fs::read_to_string(&path).unwrap_or_default()).unwrap_or_default()
}

fn save_mount_config(c: &MountConfig) -> Result<(), String> {
    let path = mount_config_path();
    if let Some(p) = path.parent() { fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    fs::write(&path, serde_json::to_string_pretty(c).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

fn cmd_auto_add(connection: &str, share: &str) {
    let Some(conn) = find_connection(connection) else {
        eprintln!("✗ 연결 '{}' 이(가) 없습니다.", connection);
        return;
    };
    let mut cfg = load_mount_config();
    if cfg.auto_mounts.iter().any(|a| a.connection == connection && a.share == share) {
        println!("이미 등록됨: {}/{}", connection, share);
        return;
    }

    // 서버에서 실제 share 목록 조회해 존재/권한 검증.
    // 비번이 없거나 smbutil 실패 시엔 확정적 판단 불가 → skip(경고만).
    if let Some(pw) = get_password(connection) {
        let shares = list_smb_shares(&conn, &pw);
        if shares.is_empty() {
            eprintln!(
                "⚠ {} 의 share 목록을 조회하지 못했습니다 (서버 다운 또는 인증 실패). 검증 없이 등록합니다.",
                connection
            );
        } else if !shares.iter().any(|s| s == share) {
            eprintln!("✗ '{}' share 가 {} 에 존재하지 않습니다.", share, connection);
            eprintln!("  사용 가능: {}", shares.join(", "));
            return;
        }
    } else {
        eprintln!(
            "⚠ {}_PASSWORD 가 없어 share 존재 검증을 건너뜁니다.",
            connection.to_uppercase()
        );
    }

    cfg.auto_mounts.push(AutoMount {
        connection: connection.into(),
        share: share.into(),
        enabled: true,
    });
    if let Err(e) = save_mount_config(&cfg) { eprintln!("✗ {}", e); return; }
    println!("✓ 자동 마운트 추가: {}/{}", connection, share);
}

fn cmd_auto_remove(connection: &str, share: &str) {
    let mut cfg = load_mount_config();
    let before = cfg.auto_mounts.len();
    cfg.auto_mounts.retain(|a| !(a.connection == connection && a.share == share));
    if cfg.auto_mounts.len() == before {
        println!("등록되지 않은 항목: {}/{}", connection, share);
        return;
    }
    if let Err(e) = save_mount_config(&cfg) { eprintln!("✗ {}", e); return; }
    println!("✓ 자동 마운트 제거: {}/{}", connection, share);
}

fn cmd_auto_toggle(connection: &str, share: &str) {
    let mut cfg = load_mount_config();
    let Some(item) = cfg.auto_mounts.iter_mut().find(|a| a.connection == connection && a.share == share) else {
        eprintln!("✗ 등록되지 않음: {}/{}", connection, share);
        return;
    };
    item.enabled = !item.enabled;
    let en = item.enabled;
    if let Err(e) = save_mount_config(&cfg) { eprintln!("✗ {}", e); return; }
    println!("{}/{} {}", connection, share, if en { "✓ 활성화" } else { "✗ 비활성화" });
}

fn cmd_auto_list() {
    let cfg = load_mount_config();
    if cfg.auto_mounts.is_empty() {
        println!("자동 마운트 설정이 없습니다.");
        println!("  mac run mount auto-add <connection> <share>");
        return;
    }
    println!("{:<10} {:<12} {:<20} {}", "STATE", "CONN", "SHARE", "MOUNTPOINT");
    println!("{}", "─".repeat(80));
    for a in &cfg.auto_mounts {
        let mp = mount_point(&a.connection, &a.share);
        let state = if !a.enabled { "✗ off" }
                    else if is_mounted_at(&mp) {
                        if is_stale(&mp) { "⚠ STALE" } else { "✓ ON" }
                    }
                    else { "○ idle" };
        println!("{:<10} {:<12} {:<20} {}", state, a.connection, a.share, mp.display());
    }
}

fn cmd_auto() {
    let cfg = load_mount_config();
    if cfg.auto_mounts.is_empty() {
        println!("자동 마운트 설정이 없습니다.");
        return;
    }
    let mut mounted_count = 0;
    let mut skipped_count = 0;
    let mut healed_count = 0;
    let mut failed_count = 0;

    for a in &cfg.auto_mounts {
        if !a.enabled { continue; }
        let mp = mount_point(&a.connection, &a.share);

        // 이미 마운트되어 있나?
        if is_mounted_at(&mp) {
            // stale 검사
            if is_stale(&mp) {
                eprintln!("  ⚠ {}/{}: stale 감지, 재마운트 시도", a.connection, a.share);
                let _ = unmount_path(&mp);
                healed_count += 1;
                // fallthrough 해서 아래에서 다시 마운트
            } else {
                skipped_count += 1;
                continue;
            }
        }

        let Some(conn) = find_connection(&a.connection) else {
            eprintln!("  ✗ {}/{}: 연결 없음", a.connection, a.share);
            failed_count += 1;
            continue;
        };
        let Some(pw) = get_password(&a.connection) else {
            eprintln!("  ✗ {}/{}: 비번 없음 (.env 의 {}_PASSWORD)", a.connection, a.share, a.connection.to_uppercase());
            failed_count += 1;
            continue;
        };

        let req = backend::MountRequest {
            host: &conn.host,
            share: &a.share,
            user: &conn.user,
            password: &pw,
            mountpoint: &mp,
        };
        let result = backend::mount(&req, |r| {
            mount_smbfs(r.user, r.password, r.host, r.share, &r.mountpoint.to_path_buf())
        });
        match result {
            Ok(backend_name) => {
                println!("  ✓ {}/{} → {} ({})", a.connection, a.share, mp.display(), backend_name);
                mounted_count += 1;
            }
            Err(e) => {
                eprintln!("  ✗ {}/{}: {}", a.connection, a.share, e);
                failed_count += 1;
            }
        }
    }
    println!("\nauto: 마운트 {}, 스킵 {}, stale-회복 {}, 실패 {}",
        mounted_count, skipped_count, healed_count, failed_count);
}

const AUTOMOUNT_LABEL: &str = "com.mac-app-init.automount";

fn automount_plist_path() -> PathBuf {
    PathBuf::from(home()).join(format!("Library/LaunchAgents/{}.plist", AUTOMOUNT_LABEL))
}

fn cmd_auto_enable() {
    let mac_bin = Command::new("which").arg("mac").output()
        .ok().and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else { None })
        .unwrap_or_else(|| "mac".into());

    let log_dir = format!("{}/문서/시스템/로그", home());
    fs::create_dir_all(&log_dir).ok();

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>run</string>
        <string>mount</string>
        <string>auto</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StartInterval</key>
    <integer>300</integer>
    <key>StandardOutPath</key>
    <string>{log_dir}/automount.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/automount.log</string>
</dict>
</plist>
"#, label=AUTOMOUNT_LABEL, bin=mac_bin, log_dir=log_dir);

    let path = automount_plist_path();
    if let Some(p) = path.parent() { fs::create_dir_all(p).ok(); }
    if let Err(e) = fs::write(&path, plist) {
        eprintln!("✗ plist 작성 실패: {}", e);
        return;
    }
    let _ = Command::new("launchctl").args(["unload", &path.to_string_lossy()]).output();
    let load = Command::new("launchctl").args(["load", &path.to_string_lossy()]).output();
    match load {
        Ok(o) if o.status.success() => println!("✓ 자동 마운트 LaunchAgent 등록 (로그인 시 + 5분마다)"),
        Ok(o) => eprintln!("✗ launchctl load: {}", String::from_utf8_lossy(&o.stderr).trim()),
        Err(e) => eprintln!("✗ {}", e),
    }
}

fn cmd_auto_disable() {
    let path = automount_plist_path();
    if !path.exists() {
        println!("자동 마운트 LaunchAgent 가 등록되어 있지 않습니다.");
        return;
    }
    let _ = Command::new("launchctl").args(["unload", &path.to_string_lossy()]).output();
    if let Err(e) = fs::remove_file(&path) {
        eprintln!("✗ 삭제 실패: {}", e);
        return;
    }
    println!("✓ 자동 마운트 LaunchAgent 제거");
}

fn cmd_unmount(target: &str) {
    // target 우선순위: 절대경로 > ~/NAS/<target> > /Volumes/<target>
    let candidates: Vec<PathBuf> = if target.starts_with('/') {
        vec![PathBuf::from(target)]
    } else {
        let mut v = Vec::new();
        // ~/NAS/<conn>/<share> 형태로 들어왔을 가능성
        let nas_path = nas_root().join(target);
        if nas_path.exists() { v.push(nas_path); }
        // 또는 share 만 들어왔으면 ~/NAS 아래에서 검색
        if let Ok(entries) = fs::read_dir(nas_root()) {
            for conn_entry in entries.flatten() {
                let p = conn_entry.path().join(target);
                if p.exists() { v.push(p); }
            }
        }
        // legacy /Volumes
        let vol = PathBuf::from(format!("/Volumes/{}", target));
        if vol.exists() { v.push(vol); }
        v
    };

    if candidates.is_empty() {
        eprintln!("✗ '{}' 에 해당하는 마운트 경로를 찾을 수 없습니다.", target);
        return;
    }
    for path in candidates {
        match unmount_path(&path) {
            Ok(()) => println!("✓ unmounted {}", path.display()),
            Err(e) => eprintln!("✗ {} — {}", path.display(), e),
        }
    }
}

fn print_tui_spec() {
    let conns = load_all_connections();
    let mounts = list_current_mounts();
    let cfg = load_mount_config();
    let auto_enabled = automount_plist_path().exists();
    let auto_rows: Vec<serde_json::Value> = cfg.auto_mounts.iter().map(|a| {
        let mp = mount_point(&a.connection, &a.share);
        let state = if !a.enabled { "off" }
                    else if is_mounted_at(&mp) {
                        if is_stale(&mp) { "⚠ STALE" } else { "✓ ON" }
                    }
                    else { "○ idle" };
        serde_json::json!([state, a.connection, a.share, mp.to_string_lossy().to_string()])
    }).collect();

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
                    },
                    {
                        "key": "자동 마운트 LaunchAgent",
                        "value": if auto_enabled { "✓ 등록됨 (5분마다)" } else { "✗ 미등록" },
                        "status": if auto_enabled { "ok" } else { "warn" }
                    },
                    {
                        "key": "자동 마운트 항목",
                        "value": format!("{}개 (활성 {}개)",
                            cfg.auto_mounts.len(),
                            cfg.auto_mounts.iter().filter(|a| a.enabled).count()
                        ),
                        "status": "ok"
                    }
                ]
            },
            {
                "kind": "table",
                "title": "자동 마운트 설정",
                "headers": ["STATE", "CONN", "SHARE", "MOUNTPOINT"],
                "rows": auto_rows
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
                    { "label": "List (현재 마운트)", "command": "list", "key": "l" },
                    { "label": "Auto (지금 자동 마운트)", "command": "auto", "key": "a" },
                    { "label": "Auto-list (설정 보기)", "command": "auto-list", "key": "i" },
                    { "label": "Auto-enable (LaunchAgent)", "command": "auto-enable", "key": "e" },
                    { "label": "Auto-disable", "command": "auto-disable", "key": "d" }
                ]
            },
            {
                "kind": "text",
                "title": "사용법 — 터미널",
                "content": "  자동 마운트 설정:\n    mac run mount auto-add <conn> <share>\n    mac run mount auto-toggle <conn> <share>\n    mac run mount auto-enable      # 로그인 시 + 5분마다 자동 실행\n\n  수동:\n    mac run mount mount <name> <share>\n    mac run mount unmount <share>"
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
