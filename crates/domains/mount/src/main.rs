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
#[command(about = "SMB/NFS 공유 마운트 관리 (env 카드 필요)")]
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
    /// NAS 잔재 자동 격리 켜기/끄기 (auto 실행 시마다)
    Sweep {
        /// on | off
        toggle: String,
    },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

use mac_common::{paths, tui_spec::{self, TuiSpec}};

fn home() -> String {
    paths::home()
}

/// 통합 마운트 루트 (~/Documents/WORK/NAS)
fn nas_root() -> PathBuf {
    PathBuf::from(home()).join("Documents/WORK/NAS")
}

/// 마운트 포인트: ~/Documents/WORK/NAS/<conn>/<share>
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
    PathBuf::from(home()).join(".mac-app-init/domains/mac-domain-env").exists()
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

fn is_mountable_scheme(scheme: &str) -> bool {
    matches!(scheme, "smb" | "nfs" | "afp" | "webdav" | "webdavs" | "rclone")
}

fn load_connections() -> Vec<Connection> {
    let path = connections_path();
    if !path.exists() { return Vec::new(); }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    let result: Vec<Connection> = json.get("services").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| {
            let scheme = s.get("scheme").and_then(|v| v.as_str()).unwrap_or("smb").to_string();
            if !is_mountable_scheme(&scheme) {
                return None;
            }
            Some(Connection {
                name: s.get("name")?.as_str()?.to_string(),
                host: s.get("host")?.as_str()?.to_string(),
                user: s.get("user")?.as_str()?.to_string(),
                port: s.get("port")?.as_u64()? as u16,
                scheme,
            })
        }).collect())
        .unwrap_or_default();
    if !result.is_empty() {
        eprintln!(
            "⚠ legacy {} 를 읽는 중. `mai run env import` 로 카드로 이관 후 파일을 삭제하세요.",
            path.display()
        );
    }
    result
}

fn find_connection(name: &str) -> Option<Connection> {
    // 1순위: env 카드. 2순위: legacy connections.json
    if let Some(c) = env_card_show(name) {
        if !is_mountable_scheme(&c.scheme) {
            return None;
        }
        return Some(c);
    }
    load_connections().into_iter().find(|c| c.name == name)
}

/// env 카드 전체 목록 + legacy connections.json 를 합친 결과 (카드 우선, 이름 중복 제거).
fn load_all_connections() -> Vec<Connection> {
    // 카드 파일 직접 읽기 — 외부 프로세스(env list) 호출 없이.
    let mut cards: Vec<Connection> = Vec::new();
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
                        if !is_mountable_scheme(&scheme) {
                            continue;
                        }
                        cards.push(Connection {
                            name: name.into(), host: host.into(), user: user.into(), port: port as u16, scheme,
                        });
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
        eprintln!("⚠ env 도메인이 필요합니다: mai install env");
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
        Commands::Sweep { toggle } => cmd_sweep(&toggle),
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
        println!("등록된 연결이 없습니다. mai run env add");
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
        eprintln!("✗ 연결 '{}' 이(가) 없습니다. mai run env list", name);
        return;
    };
    let mp = mount_point(name, share);
    let opts = card_mount_opts(name);

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

/// ~/NAS/ 전체를 재귀적으로 스캔해서 마운트/카드에 속하지 않는 잔재를
/// ~/NAS/.mountless-trash/YYMMDD-HHMMSS/ 아래에 원래 경로 구조 그대로 격리.
///
/// 보존 대상:
///   - .mountless-trash 자체
///   - 카드에 등록된 연결 이름 디렉터리 (예: synology/, truenas/)
///   - 활성 마운트 포인트 (예: synology/works/)와 그 하위 파일
///   - 자동마운트에 등록된 share 디렉터리
///
/// 격리 대상:
///   - 카드에 없는 디렉터리 (예: testafp/, unknown/)
///   - 카드 하위에서 마운트도 아니고 자동마운트도 아닌 빈 디렉터리 (예: truenas/notexist/)
///   - .DS_Store 등 dotfile
fn sweep_nas_orphans() {
    let root = nas_root();
    if !root.exists() { return; }

    let conns = load_all_connections();
    let conn_names: std::collections::HashSet<String> =
        conns.iter().map(|c| c.name.clone()).collect();

    let cfg = load_mount_config();
    // enabled=true 인 자동마운트만 보존. off 는 격리 대상.
    let auto_shares: std::collections::HashSet<String> =
        cfg.auto_mounts.iter()
            .filter(|a| a.enabled)
            .map(|a| format!("{}/{}", a.connection, a.share))
            .collect();
    // off 상태 항목은 auto-list 에서도 자동 제거.
    let disabled: Vec<_> = cfg.auto_mounts.iter()
        .filter(|a| !a.enabled)
        .map(|a| format!("{}/{}", a.connection, a.share))
        .collect();
    if !disabled.is_empty() {
        let mut cfg_mut = cfg.clone();
        cfg_mut.auto_mounts.retain(|a| a.enabled);
        let _ = save_mount_config(&cfg_mut);
        for d in &disabled {
            eprintln!("  ♻ 비활성 자동마운트 자동 제거: {}", d);
        }
    }

    let active_mounts: std::collections::HashSet<String> =
        list_current_mounts().into_iter().map(|(_, mp)| mp).collect();

    let ts = Command::new("date")
        .args(["+%y%m%d-%H%M%S"])
        .output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| format!("epoch-{}", epoch_now()));
    let trash_base = root.join(".mountless-trash").join(ts);

    let mut moved = 0;

    // 1단계: NAS 루트 직속 — 카드에 없는 항목
    let root_entries = match fs::read_dir(&root) {
        Ok(it) => it.filter_map(|e| e.ok()).collect::<Vec<_>>(),
        Err(_) => return,
    };
    for entry in &root_entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".mountless-trash" { continue; }
        if name.starts_with('.') {
            moved += move_to_trash(&entry.path(), &root, &trash_base);
            continue;
        }
        if conn_names.contains(&name) { continue; }
        moved += move_to_trash(&entry.path(), &root, &trash_base);
    }

    // 2단계: 카드 디렉터리 하위 — share 레벨 정리
    for conn_name in &conn_names {
        let conn_dir = root.join(conn_name);
        if !conn_dir.exists() || !conn_dir.is_dir() { continue; }
        let sub_entries = match fs::read_dir(&conn_dir) {
            Ok(it) => it.filter_map(|e| e.ok()).collect::<Vec<_>>(),
            Err(_) => continue,
        };
        for entry in &sub_entries {
            let share_name = entry.file_name().to_string_lossy().to_string();
            let full_key = format!("{}/{}", conn_name, share_name);
            let mp = entry.path();
            let mp_str = mp.to_string_lossy().to_string();

            // dotfile → 격리
            if share_name.starts_with('.') {
                moved += move_to_trash(&mp, &root, &trash_base);
                continue;
            }
            // 활성 마운트 → 보존
            if active_mounts.contains(&mp_str) { continue; }
            // 자동마운트 등록 → 보존 (마운트 안 됐어도 재시도 대상)
            if auto_shares.contains(&full_key) { continue; }
            // 비어있는 디렉터리 → 격리 (실패한 마운트 시도 잔재)
            if mp.is_dir() {
                let is_empty = fs::read_dir(&mp).map(|mut it| it.next().is_none()).unwrap_or(true);
                if is_empty {
                    moved += move_to_trash(&mp, &root, &trash_base);
                    continue;
                }
            }
            // 파일 → 격리
            if mp.is_file() {
                moved += move_to_trash(&mp, &root, &trash_base);
            }
        }
    }

    if moved > 0 {
        eprintln!("  🗂 NAS 잔재 {}개 → {}", moved, trash_base.display());
    }
    // trash_base 디렉터리가 비었으면 제거 (아무것도 안 옮겼을 때)
    if moved == 0 {
        let _ = fs::remove_dir(&trash_base);
    }
}

/// src 를 trash_base 아래 원래 경로 구조로 이동. 성공 시 1, 실패 시 0.
fn move_to_trash(src: &PathBuf, nas_root: &PathBuf, trash_base: &PathBuf) -> usize {
    let rel = match src.strip_prefix(nas_root) {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let dest = trash_base.join(rel);
    if let Some(parent) = dest.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("  ⚠ 디렉터리 생성 실패: {}: {}", parent.display(), e);
            return 0;
        }
    }
    if let Err(e) = fs::rename(src, &dest) {
        eprintln!("  ⚠ 이동 실패: {} → {}: {}", src.display(), dest.display(), e);
        0
    } else { 1 }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MountConfig {
    #[serde(default)]
    auto_mounts: Vec<AutoMount>,
    /// NAS 잔재 자동 격리. auto 실행 시마다 ~/Documents/WORK/NAS/ 스캔.
    /// false 면 sweep 안 함 (잔재 방치).
    #[serde(default = "default_true")]
    sweep_enabled: bool,
}

impl Default for MountConfig {
    fn default() -> Self {
        Self { auto_mounts: Vec::new(), sweep_enabled: true }
    }
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
        println!("  mai run mount auto-add <connection> <share>");
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
        if !path.starts_with(format!("{}/Documents/WORK/NAS", home())) { continue; }
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
    // [C] 사이클 시작 시 stale 일괄 청소 + NAS 잔재 정리.
    sweep_stale_mounts();
    if cfg.sweep_enabled {
        sweep_nas_orphans();
    }

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
            "마운트 실패 {}개, quarantine {}개. `mai-tui` 또는 `mai run mount auto-status` 로 확인.",
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

fn cmd_sweep(toggle: &str) {
    let mut cfg = load_mount_config();
    match toggle.to_lowercase().as_str() {
        "on" | "true" | "1" => {
            cfg.sweep_enabled = true;
            let _ = save_mount_config(&cfg);
            println!("✓ NAS 잔재 자동 격리 켜짐 (auto 실행 시마다 ~/Documents/WORK/NAS/ 스캔)");
        }
        "off" | "false" | "0" => {
            cfg.sweep_enabled = false;
            let _ = save_mount_config(&cfg);
            println!("✓ NAS 잔재 자동 격리 꺼짐");
        }
        "status" => {
            println!("sweep: {}", if cfg.sweep_enabled { "켜짐 (auto 실행 시마다)" } else { "꺼짐" });
            let trash = nas_root().join(".mountless-trash");
            if trash.exists() {
                let count = fs::read_dir(&trash).map(|it| it.count()).unwrap_or(0);
                println!(".mountless-trash: {}개 세션", count);
            }
        }
        _ => {
            eprintln!("사용법: mount sweep <on|off|status>");
            std::process::exit(1);
        }
    }
}

fn cmd_auto_enable() {
    let mac_bin = Command::new("which").arg("mac").output()
        .ok().and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else { None })
        .unwrap_or_else(|| "mac".into());

    let log_dir = format!("{}/Documents/WORK/logs", home());
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
    // tui-spec 은 가벼워야 함 — is_stale (fs::read_dir 2s 타임아웃) 호출 금지.
    // mount 목록 기반으로만 상태 판별.
    let mounted_set: std::collections::HashSet<String> = mounts.iter().map(|(_, m)| m.clone()).collect();
    let auto_items: Vec<serde_json::Value> = cfg.auto_mounts.iter().map(|a| {
        let mp = mount_point(&a.connection, &a.share);
        let mp_str = mp.to_string_lossy().to_string();
        let state = if !a.enabled { "off" }
                    else if mounted_set.contains(&mp_str) { "✓ ON" }
                    else { "○ idle" };
        tui_spec::kv_item_data(
            &format!("{}/{}", a.connection, a.share),
            &format!("{}  →  {}", state, mp.to_string_lossy()),
            if a.enabled { "ok" } else { "warn" },
            serde_json::json!({
                "name": format!("{}/{}", a.connection, a.share),
                "connection": a.connection,
                "share": a.share,
                "enabled": a.enabled.to_string(),
                "mountpoint": mp.to_string_lossy().to_string(),
            }))
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

    let active_count = cfg.auto_mounts.iter().filter(|a| a.enabled).count();
    let usage_active = !cfg.auto_mounts.is_empty();
    let usage_summary = format!("마운트 {}개 (활성 {})", cfg.auto_mounts.len(), active_count);

    TuiSpec::new("mount")
        .refresh(10)
        .list_section("자동 마운트")
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("env 도메인",
                if connect_ok { "✓ 설치됨" } else { "✗ 미설치 (mai install env)" },
                if connect_ok { "ok" } else { "error" }),
            tui_spec::kv_item("등록된 연결",
                &format!("{}개", conns.len()),
                if conns.is_empty() { "warn" } else { "ok" }),
            tui_spec::kv_item("현재 마운트",
                &format!("{}개", mounts.len()), "ok"),
            tui_spec::kv_item("자동 마운트 LaunchAgent",
                if auto_enabled { "✓ 등록됨 (5분마다)" } else { "✗ 미등록" },
                if auto_enabled { "ok" } else { "warn" }),
            tui_spec::kv_item("자동 마운트 항목",
                &format!("{}개 (활성 {}개)",
                    cfg.auto_mounts.len(),
                    cfg.auto_mounts.iter().filter(|a| a.enabled).count()),
                "ok"),
            tui_spec::kv_item("잔재 자동 정리 (sweep)",
                if cfg.sweep_enabled {
                    "✓ 켜짐 (auto 실행 시마다 ~/Documents/WORK/NAS/ 스캔)"
                } else {
                    "✗ 꺼짐 (mai run mount sweep on 으로 활성화)"
                },
                if cfg.sweep_enabled { "ok" } else { "warn" }),
        ])
        .kv("자동 마운트", auto_items)
        .table("연결", vec!["NAME", "ENDPOINT", "PW"], conn_rows)
        .table("마운트", vec!["SOURCE", "MOUNTPOINT"], mount_rows)
        .buttons()
        .buttons_custom("토글", vec![
            serde_json::json!({
                "label": if cfg.sweep_enabled { "Sweep OFF" } else { "Sweep ON" },
                "command": "sweep",
                "args": [if cfg.sweep_enabled { "off" } else { "on" }],
                "key": "w"
            }),
        ])
        .text("안내", "  자동 마운트 설정:\n    mai run mount auto-add <conn> <share>\n    mai run mount auto-toggle <conn> <share>\n    mai run mount auto-enable      # 로그인 시 + 5분마다 자동 실행\n\n  잔재 정리:\n    mai run mount sweep on|off|status\n    → auto 실행 시마다 ~/Documents/WORK/NAS/ 스캔, 카드/마운트에 없는 항목을\n      ~/NAS/.mountless-trash/YYMMDD-HHMMSS/ 로 격리 (삭제 아님)\n\n  수동:\n    mai run mount mount <name> <share>\n    mai run mount unmount <share>")
        .print();
}
