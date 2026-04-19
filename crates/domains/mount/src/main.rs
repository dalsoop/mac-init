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
    /// quarantine / backoff 상태 조회
    AutoStatus,
    /// quarantine 해제 (전부 또는 특정 share)
    AutoResume {
        /// "conn/share" 또는 생략 시 전부
        target: Option<String>,
    },
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
fn mount_smbfs(user: &str, password: &str, host: &str, share: &str, mp: &PathBuf, opts_str: &str) -> Result<(), String> {
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
            .args(["-o", opts_str])
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

#[derive(Debug, Clone)]
struct Connection {
    name: String,
    host: String,
    user: String,
    port: u16,
    #[allow(dead_code)]
    scheme: String,
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
                scheme: s.get("scheme").and_then(|v| v.as_str()).unwrap_or("smb").to_string(),
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
                                let scheme = v.get("scheme").and_then(|x| x.as_str()).unwrap_or("smb").to_string();
                                cards.push(Connection {
                                    name: name.into(), host: host.into(), user: user.into(), port: port as u16, scheme,
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
        scheme: json.get("scheme").and_then(|v| v.as_str()).unwrap_or("smb").to_string(),
    })
}

/// 카드의 mount_options. 카드 없거나 필드 없으면 backend 기본값.
fn card_mount_opts(name: &str) -> backend::MountOpts {
    let out = match Command::new(env_binary()).args(["show", name]).output() {
        Ok(o) if o.status.success() => o,
        _ => return backend::MountOpts::default(),
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return backend::MountOpts::default();
    };
    let mo = json.get("mount_options");
    let get = |k: &str, d: bool| {
        mo.and_then(|m| m.get(k)).and_then(|v| v.as_bool()).unwrap_or(d)
    };
    let get_u32 = |k: &str| -> u32 {
        mo.and_then(|m| m.get(k)).and_then(|v| v.as_u64()).unwrap_or(0) as u32
    };
    backend::MountOpts {
        readonly: get("readonly", false),
        noappledouble: get("noappledouble", true),
        soft: get("soft", true),
        nobrowse: get("nobrowse", true),
        rsize: get_u32("rsize"),
        wsize: get_u32("wsize"),
    }
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
    list_all_mounts().into_iter().map(|(s, m, _)| (s, m)).collect()
}

/// 모든 네트워크 마운트 (source, mountpoint, fs_type).
/// SMB/NFS/macFUSE/AFP/SSHFS/WebDAV 전부 인식.
fn list_all_mounts() -> Vec<(String, String, String)> {
    let out = Command::new("mount").output();
    let Ok(o) = out else { return Vec::new(); };
    String::from_utf8_lossy(&o.stdout).lines()
        .filter_map(|l| {
            // 형식: "<source> on <mountpoint> (<fstype>, ...)"
            let on_idx = l.find(" on ")?;
            let source = &l[..on_idx];
            let rest = &l[on_idx + 4..];
            let paren = rest.find(" (")?;
            let mountpoint = &rest[..paren];
            let opts = &rest[paren + 2..];
            let close = opts.find(')').unwrap_or(opts.len());
            let fstype = opts[..close].split(',').next().unwrap_or("").trim().to_string();
            let net_types = ["smbfs", "nfs", "afpfs", "macfuse", "osxfuse", "fuse", "webdav"];
            if !net_types.iter().any(|t| fstype.contains(t)) { return None; }
            Some((source.to_string(), mountpoint.to_string(), fstype))
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
        Commands::AutoStatus => cmd_auto_status(),
        Commands::AutoResume { target } => cmd_auto_resume(target.as_deref()),
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
    let mounts = list_all_mounts();
    if mounts.is_empty() {
        println!("현재 마운트된 네트워크 공유가 없습니다.");
        return;
    }
    println!("{:<10} {:<40} {}", "TYPE", "SOURCE", "MOUNTPOINT");
    println!("{}", "─".repeat(90));
    for (src, mp, ft) in mounts {
        println!("{:<10} {:<40} {}", ft, src, mp);
    }
}

fn cmd_mount(name: &str, share: &str) {
    let Some(conn) = find_connection(name) else {
        eprintln!("✗ 연결 '{}' 이(가) 없습니다. mac run connect list", name);
        return;
    };
    let mp = mount_point(name, share);
    let opts = card_mount_opts(name);

    // 마운트 전 잔재 파일 격리
    sweep_mountless_files(&mp, name, share);

    // rclone 백엔드 분기
    if conn.scheme == "rclone" {
        let (remote, path) = card_rclone_meta(name);
        let path = if share == "/" || share.is_empty() { path } else { share.into() };
        println!("마운트 중 (rclone): {}:{} → {}", remote, path, mp.display());
        match backend::rclone_mount(&remote, &path, &mp, &opts) {
            Ok(()) => println!("✓ {} (rclone)", mp.display()),
            Err(e) => eprintln!("✗ {}", e),
        }
        return;
    }

    // 기존: SMB / NFS / AFP / WebDAV 등 NetFS 경로
    let Some(pw) = get_password(name) else {
        eprintln!("✗ {}_PASSWORD 가 .env 에 없습니다.", name.to_uppercase());
        return;
    };
    println!("마운트 중: {} → {}", conn.host, mp.display());
    let opts_str = opts.smbfs_opts_string();
    let req = backend::MountRequest {
        host: &conn.host,
        share,
        user: &conn.user,
        password: &pw,
        mountpoint: &mp,
        opts,
        scheme: &conn.scheme,
        port: conn.port,
    };
    let result = backend::mount(&req, |r| {
        mount_smbfs(r.user, r.password, r.host, r.share, &r.mountpoint.to_path_buf(), &opts_str)
    });
    match result {
        Ok(backend_name) => println!("✓ {} ({}) [-o {}]", mp.display(), backend_name, opts_str),
        Err(e) => eprintln!("✗ {}", e),
    }
}

/// 마운트 포인트에 마운트 전 남은 로컬 잔재 파일을 격리.
/// ~/NAS/.mountless-trash/YYMMDD-HHMMSS-<conn>-<share>/ 로 이동.
///
/// **안전장치**: 이미 마운트된 경로에는 실행하지 않음 (원격 파일 이동 방지).
/// TOCTOU 윈도우(read_dir ↔ rename 사이 새 파일 생성)는 best-effort 허용.
fn sweep_mountless_files(mp: &PathBuf, conn: &str, share: &str) {
    if !mp.exists() { return; }
    // 이미 마운트된 경로면 절대 건드리지 않음
    if is_mounted_at(mp) { return; }

    let entries: Vec<_> = match fs::read_dir(mp) {
        Ok(it) => it.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };
    if entries.is_empty() { return; }

    let ts = Command::new("date")
        .args(["+%y%m%d-%H%M%S"])
        .output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| format!("epoch-{}", epoch_now()));

    let safe_share = share.replace('/', "_");
    let trash_dir = nas_root().join(".mountless-trash")
        .join(format!("{}-{}-{}", ts, conn, safe_share));

    if let Err(e) = fs::create_dir_all(&trash_dir) {
        eprintln!("  ⚠ mountless-trash 생성 실패: {}", e);
        return;
    }

    let mut moved = 0;
    for entry in entries {
        let name = entry.file_name();
        let dest = trash_dir.join(&name);
        if let Err(e) = fs::rename(entry.path(), &dest) {
            eprintln!("  ⚠ 이동 실패: {} → {}: {}", entry.path().display(), dest.display(), e);
        } else {
            moved += 1;
        }
    }
    if moved > 0 {
        eprintln!("  🗂 마운트 전 잔재 {}개 → {}", moved, trash_dir.display());
    }
}

/// rclone 카드의 (remote, path) 메타. env show 의 rclone_remote/rclone_path 필드.
fn card_rclone_meta(name: &str) -> (String, String) {
    let out = Command::new(env_binary()).args(["show", name]).output();
    let Ok(o) = out else { return (String::new(), String::new()); };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&o.stdout) else {
        return (String::new(), String::new());
    };
    let r = json.get("rclone_remote").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let p = json.get("rclone_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
    (r, p)
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

/// macOS 알림 센터에 메시지 표시. LaunchAgent 백그라운드에서도 동작.
fn notify(title: &str, msg: &str) {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        msg.replace('"', "\\\""),
        title.replace('"', "\\\""),
    );
    let _ = Command::new("osascript").args(["-e", &script]).output();
}

/// host:port 에 1초 안에 TCP 연결 가능한지. precheck 용.
fn host_reachable(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    use std::time::Duration;
    let addr = format!("{}:{}", host, port);
    if let Ok(mut it) = addr.to_socket_addrs() {
        if let Some(sock) = it.next() {
            return std::net::TcpStream::connect_timeout(&sock, Duration::from_secs(1)).is_ok();
        }
    }
    false
}

/// auto 사이클 시작 시 stale 마운트 일괄 청소. 좀비 마운트 누적 방지.
fn sweep_stale_mounts() -> usize {
    let mut swept = 0;
    for (_, mp, _) in list_all_mounts() {
        let path = PathBuf::from(&mp);
        if !path.starts_with(format!("{}/NAS", home())) { continue; }
        if is_stale(&path) {
            if unmount_path(&path).is_ok() {
                eprintln!("  🧹 stale 청소: {}", mp);
                swept += 1;
            }
        }
    }
    swept
}

fn cmd_auto() {
    let cfg = load_mount_config();
    if cfg.auto_mounts.is_empty() {
        println!("자동 마운트 설정이 없습니다.");
        return;
    }
    // [C] 사이클 시작 시 stale 일괄 청소.
    sweep_stale_mounts();

    let mut state = load_retry_state();
    let now = epoch_now();

    let mut mounted_count = 0;
    let mut skipped_count = 0;
    let mut healed_count = 0;
    let mut failed_count = 0;
    let mut quarantined_count = 0;

    for a in &cfg.auto_mounts {
        if !a.enabled { continue; }
        let key = format!("{}/{}", a.connection, a.share);
        let mp = mount_point(&a.connection, &a.share);

        // quarantine / backoff 체크
        if let Some(rs) = state.shares.get(&key) {
            if rs.quarantined {
                quarantined_count += 1;
                continue;
            }
            if rs.next_attempt_at > now {
                let wait = rs.next_attempt_at - now;
                eprintln!("  ⏸ {}/{}: backoff (재시도 {}초 후, 누적 실패 {})", a.connection, a.share, wait, rs.failures);
                continue;
            }
        }

        if is_mounted_at(&mp) {
            if is_stale(&mp) {
                eprintln!("  ⚠ {}/{}: stale 감지, 재마운트 시도", a.connection, a.share);
                let _ = unmount_path(&mp);
                healed_count += 1;
            } else {
                skipped_count += 1;
                state.shares.remove(&key); // 성공 상태면 카운터 리셋
                continue;
            }
        }

        let Some(conn) = find_connection(&a.connection) else {
            eprintln!("  ✗ {}/{}: 연결 없음", a.connection, a.share);
            failed_count += 1;
            record_failure(&mut state, &key, now, "no_connection");
            continue;
        };

        // [A] precheck: host TCP probe — 도달 불가하면 mount 시도 자체 skip.
        // VPN 다운/네트워크 단절 상황에서 시도 폭주 + 로그 폭증 방지.
        if conn.scheme != "rclone" && !host_reachable(&conn.host, conn.port) {
            eprintln!("  ⌧ {}/{}: {}:{} 도달 불가 (네트워크/VPN 확인) — skip",
                a.connection, a.share, conn.host, conn.port);
            failed_count += 1;
            record_failure(&mut state, &key, now, "unreachable");
            continue;
        }

        let Some(pw) = get_password(&a.connection) else {
            eprintln!("  ✗ {}/{}: 비번 없음 (.env 의 {}_PASSWORD)", a.connection, a.share, a.connection.to_uppercase());
            failed_count += 1;
            record_failure(&mut state, &key, now, "no_password");
            continue;
        };

        sweep_mountless_files(&mp, &a.connection, &a.share);

        let opts = card_mount_opts(&a.connection);
        let opts_str = opts.smbfs_opts_string();
        let req = backend::MountRequest {
            host: &conn.host,
            share: &a.share,
            user: &conn.user,
            password: &pw,
            mountpoint: &mp,
            opts,
            scheme: &conn.scheme,
            port: conn.port,
        };
        let result = backend::mount(&req, |r| {
            mount_smbfs(r.user, r.password, r.host, r.share, &r.mountpoint.to_path_buf(), &opts_str)
        });
        match result {
            Ok(backend_name) => {
                println!("  ✓ {}/{} → {} ({})", a.connection, a.share, mp.display(), backend_name);
                mounted_count += 1;
                state.shares.remove(&key);
            }
            Err(e) => {
                eprintln!("  ✗ {}/{}: {}", a.connection, a.share, e);
                failed_count += 1;
                let perm = is_permanent_failure(&e);
                record_failure(&mut state, &key, now, if perm { "EACCES" } else { "transient" });
            }
        }
    }
    let _ = save_retry_state(&state);
    println!("\nauto: 마운트 {}, 스킵 {}, stale-회복 {}, 실패 {}, quarantine {}",
        mounted_count, skipped_count, healed_count, failed_count, quarantined_count);

    // 실패가 있으면 macOS 알림.
    if failed_count > 0 || quarantined_count > 0 {
        let msg = format!(
            "마운트 실패 {}개, quarantine {}개. `mac-tui` 또는 `mac run mount auto-status` 로 확인.",
            failed_count, quarantined_count
        );
        notify("mac-app-init: Mount", &msg);
    }
}

fn cmd_auto_status() {
    let s = load_retry_state();
    if s.shares.is_empty() {
        println!("retry state 없음 (모든 자동마운트 정상).");
        return;
    }
    let now = epoch_now();
    println!("{:<28} {:<6} {:<10} {:<14} REASON", "TARGET", "FAILS", "STATE", "NEXT");
    println!("{}", "─".repeat(80));
    for (k, r) in &s.shares {
        let state = if r.quarantined { "🔒 quar" } else { "↻ retry" };
        let next = if r.quarantined { "—".into() }
            else if r.next_attempt_at <= now { "now".into() }
            else { format!("+{}s", r.next_attempt_at - now) };
        println!("{:<28} {:<6} {:<10} {:<14} {}", k, r.failures, state, next, r.last_reason);
    }
}

fn cmd_auto_resume(target: Option<&str>) {
    let mut s = load_retry_state();
    let mut released = 0;
    let keys: Vec<String> = s.shares.keys().cloned().collect();
    for k in keys {
        if let Some(t) = target { if k != t { continue; } }
        s.shares.remove(&k);
        released += 1;
        println!("  ✓ resume: {}", k);
    }
    if released == 0 { println!("해당 항목 없음."); return; }
    if let Err(e) = save_retry_state(&s) { eprintln!("✗ 저장 실패: {}", e); }
    else { println!("\n{} 개 항목 해제됨.", released); }
}

// === retry-state ===

#[derive(Debug, Default, Serialize, Deserialize)]
struct RetryState {
    #[serde(default)]
    shares: HashMap<String, RetryRecord>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RetryRecord {
    failures: u32,
    next_attempt_at: u64,
    last_reason: String,
    #[serde(default)]
    quarantined: bool,
}

fn retry_state_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/mount-retry-state.json")
}

fn load_retry_state() -> RetryState {
    let p = retry_state_path();
    if !p.exists() { return RetryState::default(); }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save_retry_state(s: &RetryState) -> Result<(), String> {
    let p = retry_state_path();
    if let Some(parent) = p.parent() { fs::create_dir_all(parent).ok(); }
    fs::write(&p, serde_json::to_string_pretty(s).unwrap_or_default())
        .map_err(|e| format!("{}", e))
}

fn epoch_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// 영구 실패로 보이는 에러 (권한 거부 계열). 무한 재시도하면 안 됨.
fn is_permanent_failure(err: &str) -> bool {
    let l = err.to_lowercase();
    l.contains("eacces") || l.contains("permission denied")
        || l.contains("access denied") || l.contains("logon_failure")
        || l.contains("not_found") // share 자체가 없는 경우
}

/// 실패 기록 + 다음 시도 시각 계산 (exponential backoff + full jitter, max 1h).
fn record_failure(state: &mut RetryState, key: &str, now: u64, reason: &str) {
    let entry = state.shares.entry(key.to_string()).or_default();
    entry.failures += 1;
    entry.last_reason = reason.into();

    // 영구 실패가 3회 이상 누적되면 quarantine
    if reason == "EACCES" && entry.failures >= 3 {
        entry.quarantined = true;
        entry.next_attempt_at = u64::MAX;
        eprintln!("    🔒 {} quarantine (사유: 권한 거부 {}회). `mount auto-resume {}` 로 해제.",
            key, entry.failures, key);
        return;
    }

    // exponential backoff: 30s, 60s, 120s, ... max 3600s + ±25% jitter
    let base = (30u64).saturating_mul(1 << entry.failures.min(7).saturating_sub(1));
    let cap = base.min(3600);
    let jitter = (cap / 4).max(1);
    let pseudo_rand = (now ^ (entry.failures as u64).wrapping_mul(2654435761)) % (jitter * 2 + 1);
    let delay = cap.saturating_sub(jitter).saturating_add(pseudo_rand);
    entry.next_attempt_at = now.saturating_add(delay);
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
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/sbin:{home}/.cargo/bin</string>
        <key>HOME</key>
        <string>{home}</string>
    </dict>
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
    <key>ThrottleInterval</key>
    <integer>300</integer>
    <key>StandardOutPath</key>
    <string>{log_dir}/automount.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/automount.log</string>
</dict>
</plist>
"#, label=AUTOMOUNT_LABEL, bin=mac_bin, log_dir=log_dir, home=home());

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
    let auto_items: Vec<serde_json::Value> = cfg.auto_mounts.iter().map(|a| {
        let mp = mount_point(&a.connection, &a.share);
        let state = if !a.enabled { "off" }
                    else if is_mounted_at(&mp) {
                        if is_stale(&mp) { "⚠ STALE" } else { "✓ ON" }
                    }
                    else { "○ idle" };
        serde_json::json!({
            "key": format!("{}/{}", a.connection, a.share),
            "value": format!("{}  →  {}", state, mp.to_string_lossy()),
            "status": if a.enabled { "ok" } else { "warn" },
            "data": {
                "name": format!("{}/{}", a.connection, a.share),
                "connection": a.connection,
                "share": a.share,
                "enabled": a.enabled.to_string(),
                "mountpoint": mp.to_string_lossy().to_string(),
            }
        })
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
        "list_section": "자동 마운트",
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
                "kind": "key-value",
                "title": "자동 마운트",
                "items": auto_items
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
        ],
        "keybindings": [
            { "key": "T", "label": "토글(활성/비활성)",
              "command": "auto-toggle",
              "args": ["${selected.connection}", "${selected.share}"] },
            { "key": "M", "label": "수동 마운트",
              "command": "mount",
              "args": ["${selected.connection}", "${selected.share}"] },
            { "key": "U", "label": "언마운트",
              "command": "unmount",
              "args": ["${selected.mountpoint}"] },
            { "key": "X", "label": "자동마운트 항목 제거",
              "command": "auto-remove",
              "args": ["${selected.connection}", "${selected.share}"],
              "confirm": true },
            { "key": "P", "label": "quarantine 해제",
              "command": "auto-resume",
              "args": ["${selected.name}"] }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
