use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-sd-backup")]
#[command(about = "SD 카드 미디어 자동 백업 (기기별·날짜별 정리)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// SD 카드 감지 + 백업 상태
    Status,
    /// 백업 실행 (감지된 SD 카드 → 백업 대상)
    Run,
    /// 로컬 백업 경로 설정
    SetTarget { path: String },
    /// NAS 동기화 경로 설정
    SetSync { path: String },
    /// NAS 동기화 on/off
    Sync { toggle: String },
    /// 백업 완료 후 SD 자동 추출 on/off
    Eject { toggle: String },
    /// 자동 감지 모드 on/off (30초마다 /Volumes/ 스캔)
    Auto {
        /// on | off | status
        toggle: String,
    },
    /// 감시 루프 (LaunchAgent 에서 호출 — 내부용)
    Watch,
    /// 현재 백업 진행률 조회
    Progress,
    /// 기기 프로필 목록
    Devices,
    /// 백업 이력 조회
    History,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    // 숨은 모드: 자식 프로세스에서 카드 감지 → JSON stdout
    if std::env::args().any(|a| a == "--detect-cards-json") {
        let cards = detect_cards_inner();
        println!("{}", serde_json::to_string(&cards).unwrap_or_default());
        return;
    }
    // 숨은 모드: tui-spec용 경량 카드 감지 (D-state 격리용)
    if std::env::args().any(|a| a == "detect-cards-light-internal") {
        detect_cards_light_inner();
        return;
    }

    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Run => cmd_run(),
        Commands::SetTarget { path } => cmd_set_target(&path),
        Commands::SetSync { path } => cmd_set_sync(&path),
        Commands::Sync { toggle } => cmd_sync_toggle(&toggle),
        Commands::Eject { toggle } => cmd_eject_toggle(&toggle),
        Commands::Auto { toggle } => cmd_auto(&toggle),
        Commands::Watch => cmd_watch(),
        Commands::Progress => cmd_progress(),
        Commands::Devices => cmd_devices(),
        Commands::History => cmd_history(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

// === 설정 ===

use mac_common::{paths, tui_spec::{self, TuiSpec}};

fn home() -> String { paths::home() }
fn config_path() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/sd-backup.json") }
fn history_path() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/sd-backup-history.json") }

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    /// 로컬 백업 대상 루트 경로
    #[serde(default)]
    backup_target: String,
    /// NAS 동기화 경로 (비어있으면 동기화 안 함)
    #[serde(default)]
    sync_target: String,
    /// NAS 동기화 활성 (경로 있어도 off 가능)
    #[serde(default)]
    sync_enabled: bool,
    /// 백업+동기화 완료 후 SD 카드 자동 추출
    #[serde(default)]
    auto_eject: bool,
    /// LRF(저해상도 프리뷰) 파일 포함 여부
    #[serde(default)]
    include_lrf: bool,
    /// 제외 확장자
    #[serde(default)]
    exclude_extensions: Vec<String>,
    /// 자동 감지 모드 (SD 꽂으면 자동 백업)
    #[serde(default)]
    auto_enabled: bool,
}

/// 백업 진행 상태 (실시간 파일)
#[derive(Debug, Default, Serialize, Deserialize)]
struct ProgressState {
    running: bool,
    device: String,
    current_file: String,
    files_done: usize,
    files_total: usize,
    bytes_done: u64,
    bytes_total: u64,
    started_at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BackupHistory {
    #[serde(default)]
    entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    timestamp: String,
    device: String,
    volume: String,
    files_copied: usize,
    #[serde(default)]
    files_skipped: usize,
    bytes_copied: u64,
    #[serde(default)]
    duration_secs: u64,
    #[serde(default)]
    speed_bps: u64,
    target_dir: String,
    #[serde(default)]
    files: Vec<FileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileRecord {
    name: String,
    size: u64,
    date: String,
}

fn load_config() -> Config {
    let p = config_path();
    if !p.exists() { return Config::default(); }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save_config(c: &Config) {
    let p = config_path();
    if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&p, serde_json::to_string_pretty(c).unwrap_or_default());
}

fn load_history() -> BackupHistory {
    let p = history_path();
    if !p.exists() { return BackupHistory::default(); }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save_history(h: &BackupHistory) {
    let p = history_path();
    let _ = fs::write(&p, serde_json::to_string_pretty(h).unwrap_or_default());
}

fn progress_path() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/sd-backup-progress.json") }

const LAUNCH_LABEL: &str = "com.mac-app-init.sd-backup-watch";

fn plist_path() -> PathBuf {
    PathBuf::from(home()).join(format!("Library/LaunchAgents/{}.plist", LAUNCH_LABEL))
}

/// 원자적 쓰기: tmp 파일 → rename (APFS 에서 atomic).
fn save_progress(p: &ProgressState) {
    let path = progress_path();
    let tmp = path.with_extension("tmp");
    if let Ok(json) = serde_json::to_string_pretty(p) {
        if fs::write(&tmp, &json).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }
}

fn load_progress() -> ProgressState {
    let p = progress_path();
    if !p.exists() { return ProgressState::default(); }
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn clear_progress() {
    let _ = fs::remove_file(progress_path());
    let _ = fs::remove_file(progress_path().with_extension("tmp"));
}

fn lock_path() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/sd-backup.lock") }

/// PID 기반 잠금. 이미 다른 백업이 진행 중이면 false.
fn acquire_lock() -> bool {
    let lp = lock_path();
    if lp.exists() {
        if let Ok(pid_str) = fs::read_to_string(&lp) {
            let pid = pid_str.trim();
            // 프로세스 살아있는지
            if Command::new("kill").args(["-0", pid]).status().map(|s| s.success()).unwrap_or(false) {
                return false; // 아직 실행 중
            }
        }
        // stale lock
        let _ = fs::remove_file(&lp);
    }
    let _ = fs::write(&lp, std::process::id().to_string());
    true
}

fn release_lock() {
    let _ = fs::remove_file(lock_path());
}

fn expand(p: &str) -> String { paths::expand(p) }

fn now_str() -> String {
    Command::new("date").args(["+%Y-%m-%d %H:%M:%S"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn date_str() -> String {
    Command::new("date").args(["+%Y-%m-%d"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

// === 기기 감지 ===

#[derive(Debug, Serialize, Deserialize)]
struct DetectedCard {
    volume_path: PathBuf,
    volume_name: String,
    device_type: String,        // "DJI-Action-Pro-5", "Generic-Camera", ...
    dcim_path: Option<PathBuf>,
    file_count: usize,
    total_bytes: u64,
}

/// /Volumes/ 스캔해서 DCIM 있는 볼륨 감지 + 기기 판별
/// SD 카드 감지. 별도 프로세스로 격리 — D 상태 전파 차단.
/// 자식 프로세스가 SD I/O에 stuck돼도 부모(mai-tui/watch)는 안전.
fn detect_cards() -> Vec<DetectedCard> {
    use std::time::Duration;

    // 자기 자신을 자식으로 spawn + 특수 인자
    let bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mac-domain-sd-backup"));
    let child = Command::new(&bin)
        .arg("--detect-cards-json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();

    let Ok(mut child) = child else { return detect_cards_inner(); };

    // 10초 대기
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(10) {
                    let _ = child.kill();
                    eprintln!("⚠ SD 카드 스캔 timeout (10초).");
                    return Vec::new();
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return Vec::new(),
        }
    }

    if let Some(stdout) = child.stdout.take() {
        use std::io::Read;
        let mut buf = String::new();
        let mut reader = std::io::BufReader::new(stdout);
        let _ = reader.read_to_string(&mut buf);
        serde_json::from_str(&buf).unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn detect_cards_inner() -> Vec<DetectedCard> {
    let volumes = Path::new("/Volumes");
    let Ok(entries) = fs::read_dir(volumes) else { return Vec::new(); };
    let skip = ["Macintosh HD", "Preboot", "Recovery", "VM", "Data"];

    let mut cards = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if skip.iter().any(|s| name == *s) { continue; }
        let vol = entry.path();
        let dcim = vol.join("DCIM");
        if !dcim.exists() { continue; }

        let device_type = detect_device_type(&vol);
        let (file_count, total_bytes) = count_media(&dcim);

        cards.push(DetectedCard {
            volume_path: vol,
            volume_name: name,
            device_type,
            dcim_path: Some(dcim),
            file_count,
            total_bytes,
        });
    }
    cards
}

/// 폴더 구조로 기기 종류 판별
fn detect_device_type(vol: &Path) -> String {
    let dcim = vol.join("DCIM");
    let vol_name = vol.file_name().unwrap_or_default().to_string_lossy().to_uppercase();

    // 볼륨 이름으로 빠른 판별
    if vol_name.contains("EOS") { return detect_canon(&dcim, &vol_name); }

    // DCIM 하위 폴더명으로 판별 (Canon/Nikon/GoPro 등 — DJI/MISC 보다 먼저)
    if let Ok(it) = fs::read_dir(&dcim) {
        for e in it.filter_map(|x| x.ok()) {
            let n = e.file_name().to_string_lossy().to_string().to_uppercase();
            if n.contains("CANON") || n.starts_with("CANONMSC") { return detect_canon(&dcim, &vol_name); }
            if n.contains("NIKON") || n.starts_with("NIKON") { return "Nikon".into(); }
            if n.contains("GOPR") || n.contains("HERO") { return "GoPro".into(); }
            if n.starts_with("FUJI") { return "Fujifilm".into(); }
        }
    }

    // Sony: PRIVATE/M4ROOT/
    if vol.join("PRIVATE/M4ROOT").exists() { return "Sony".into(); }

    // DJI: DCIM/DJI_NNN/ + MISC/AC*.db (MISC만으로 판별 안 함)
    if vol.join("DCIM/DJI_001").exists() {
        if vol.join("MISC/AC004.db").exists() { return "DJI-Action-Pro-5".into(); }
        if vol.join("MISC/AC003.db").exists() { return "DJI-Action-4".into(); }
        return "DJI".into();
    }
    "Generic-Camera".into()
}

/// Canon 모델 판별: DCIM/100EOSR6 → Canon-EOS-R6
fn detect_canon(dcim: &Path, vol_name: &str) -> String {
    if let Ok(it) = fs::read_dir(dcim) {
        for e in it.filter_map(|x| x.ok()) {
            let n = e.file_name().to_string_lossy().to_string().to_uppercase();
            // 100EOSR6, 101EOS5D, 100CANON 등
            if n.contains("EOSR6") { return "Canon-EOS-R6".into(); }
            if n.contains("EOSR5") { return "Canon-EOS-R5".into(); }
            if n.contains("EOSR3") { return "Canon-EOS-R3".into(); }
            if n.contains("EOSR7") { return "Canon-EOS-R7".into(); }
            if n.contains("EOSR") { return "Canon-EOS-R".into(); }
            if n.contains("EOS5D") { return "Canon-5D".into(); }
        }
    }
    if vol_name.contains("EOS") { return "Canon-EOS".into(); }
    "Canon".into()
}

/// tui-spec 용 초경량 감지: /Volumes/ 에 DCIM 있는지만. 파일 스캔 안 함.
fn detect_cards_light() -> String {
    // D-state 방어: 전체 감지를 별도 프로세스로 격리.
    // SD 카드/NAS 볼륨이 D-state(uninterruptible sleep)에 빠지면
    // 해당 볼륨에 접근하는 스레드는 물론 프로세스 exit까지 블로킹됨.
    // 별도 프로세스로 실행하고, 타임아웃 시 kill하면 메인 프로세스는 안전.
    //
    // 주의: Rust 2024 에디션에서 Child drop 시 자동 kill+wait 호출.
    // D-state child는 wait 블로킹 → mem::forget으로 leak 필수.
    let bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mac-domain-sd-backup"));
    let child = Command::new(&bin)
        .arg("detect-cards-light-internal")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return "미감지".into() };
    match wait_child_timeout(&mut child, 3) {
        Some(status) if status.success() => {
            let result = child.stdout.take().map(|mut o| {
                let mut s = String::new();
                std::io::Read::read_to_string(&mut o, &mut s).ok();
                s.trim().to_string()
            }).unwrap_or_else(|| "미감지".into());
            result
        }
        _ => {
            // 타임아웃: D-state child는 wait 블로킹하므로 Child를 leak.
            std::mem::forget(child);
            "미감지 (타임아웃)".into()
        }
    }
}

/// detect-cards-light-internal: 실제 볼륨 스캔 (자식 프로세스에서만 호출)
fn detect_cards_light_inner() {
    let volumes = Path::new("/Volumes");
    let Ok(entries) = fs::read_dir(volumes) else { println!("미감지"); return; };
    let skip = ["Macintosh HD", "Preboot", "Recovery", "VM", "Data"];
    let net_vols = network_mounted_volumes();

    let mut found = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if skip.iter().any(|s| name == *s) { continue; }
        if net_vols.contains(&name) { continue; }
        let vol = entry.path();
        if vol.join("DCIM").exists() {
            let device = detect_device_type(&vol);
            found.push(format!("{} ({})", name, device));
        }
    }
    if found.is_empty() { println!("미감지"); } else { println!("{}", found.join(", ")); }
}

/// Child 프로세스를 타임아웃으로 대기. 타임아웃 시 kill + None 반환.
/// D-state child는 kill/wait 모두 블로킹되므로, wait() 호출하지 않음.
fn wait_child_timeout(child: &mut std::process::Child, timeout_secs: u64) -> Option<std::process::ExitStatus> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

/// /Volumes 하위 네트워크 마운트 볼륨 이름 목록.
fn network_mounted_volumes() -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    let Ok(out) = Command::new("mount").output() else { return set; };
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        // "//user@host/share on /Volumes/xxx (smbfs, ...)"
        // "host:/path on /Volumes/xxx (macfuse, ...)"
        let Some(on_idx) = line.find(" on ") else { continue; };
        let rest = &line[on_idx + 4..];
        let Some(paren) = rest.find(" (") else { continue; };
        let mp = &rest[..paren];
        let fs_type = &rest[paren + 2..].split(',').next().unwrap_or("");
        let net_types = ["smbfs", "nfs", "macfuse", "sshfs", "afpfs", "webdav"];
        if net_types.iter().any(|t| fs_type.contains(t)) {
            if let Some(name) = mp.strip_prefix("/Volumes/") {
                set.insert(name.to_string());
            }
        }
    }
    set
}

fn count_media(dcim: &Path) -> (usize, u64) {
    let mut count = 0usize;
    let mut bytes = 0u64;
    if let Ok(output) = Command::new("find").args([dcim.to_str().unwrap_or("."), "-type", "f"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.is_empty() { continue; }
            count += 1;
            bytes += fs::metadata(line).map(|m| m.len()).unwrap_or(0);
        }
    }
    (count, bytes)
}

/// 단일 파일의 스테이징 복사 결과.
#[derive(Debug, PartialEq)]
enum CopyResult {
    /// 이미 존재하고 크기 동일 → skip
    Skipped,
    /// .partial → 크기 검증 → 정식 위치 이동 성공
    Copied(u64),
    /// 크기 불일치 — .partial/에 잔류
    SizeMismatch { expected: u64, actual: u64 },
    /// src 접근 불가 (SD 제거)
    SourceGone,
    /// rsync 실패
    RsyncFailed,
    /// rename 실패
    MoveFailed,
}

/// 단일 파일을 .partial/ 스테이징 경유로 복사.
/// src → staging_dir/파일명 → 크기 검증 → dest_dir/파일명
fn copy_staged(src: &Path, dest_dir: &Path, staging_dir: &Path) -> CopyResult {
    let fname = match src.file_name() {
        Some(f) => f,
        None => return CopyResult::RsyncFailed,
    };
    let dst = dest_dir.join(fname);

    // 이미 존재하고 크기 동일 → skip
    if dst.exists() {
        let src_size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        let dst_size = fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
        if src_size == dst_size {
            return CopyResult::Skipped;
        }
    }

    // src 접근 가능 확인
    let file_size = match fs::metadata(src) {
        Ok(m) => m.len(),
        Err(_) => return CopyResult::SourceGone,
    };

    let _ = fs::create_dir_all(staging_dir);
    let staging_file = staging_dir.join(fname);

    // rsync --partial로 복사 (이어받기 지원)
    let status = Command::new("rsync")
        .args(["-a", "--partial", "--progress"])
        .arg(src)
        .arg(staging_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            let staged_size = fs::metadata(&staging_file).map(|m| m.len()).unwrap_or(0);
            if staged_size != file_size {
                return CopyResult::SizeMismatch { expected: file_size, actual: staged_size };
            }
            // 정식 위치로 이동
            let _ = fs::create_dir_all(dest_dir);
            match fs::rename(&staging_file, &dst) {
                Ok(()) => CopyResult::Copied(file_size),
                Err(_) => CopyResult::MoveFailed,
            }
        }
        _ => CopyResult::RsyncFailed,
    }
}

/// .partial/ 잔류 파일 수 (하위 전체 재귀).
fn count_partial_files(partial_root: &Path) -> usize {
    if !partial_root.exists() { return 0; }
    let mut count = 0;
    fn walk(dir: &Path, count: &mut usize) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() { walk(&path, count); }
            else { *count += 1; }
        }
    }
    walk(partial_root, &mut count);
    count
}

fn human_bytes(b: u64) -> String {
    if b < 1024 { return format!("{}B", b); }
    if b < 1024 * 1024 { return format!("{:.1}KB", b as f64 / 1024.0); }
    if b < 1024 * 1024 * 1024 { return format!("{:.1}MB", b as f64 / (1024.0 * 1024.0)); }
    format!("{:.1}GB", b as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// 파일명에서 날짜 추출. DJI: DJI_YYYYMMDD..., 일반: EXIF 날짜 또는 mtime.
fn extract_date(filename: &str) -> String {
    // DJI 패턴: DJI_20260310123527_...
    if filename.starts_with("DJI_") && filename.len() > 12 {
        let d = &filename[4..12]; // YYYYMMDD
        if let (Ok(y), Ok(m), Ok(day)) = (d[0..4].parse::<u32>(), d[4..6].parse::<u32>(), d[6..8].parse::<u32>()) {
            return format!("{:04}-{:02}-{:02}", y, m, day);
        }
    }
    // GoPro 패턴: GOPRNNNN.MP4 → mtime fallback
    // 일반: mtime
    "unknown".into()
}

// === 커맨드 ===

fn cmd_status() {
    let cfg = load_config();
    println!("=== SD 미디어 백업 ===\n");

    // 백업 대상
    if cfg.backup_target.is_empty() {
        println!("⚠ 백업 대상 미설정. `mai run sd-backup set-target <경로>` 필요");
    } else {
        let expanded = expand(&cfg.backup_target);
        let exists = Path::new(&expanded).exists();
        println!("백업 대상: {} {}", cfg.backup_target, if exists { "✓" } else { "✗ 경로 없음 (마운트 확인)" });
    }
    println!("LRF 포함: {}", if cfg.include_lrf { "예" } else { "아니오" });
    println!();

    // SD 카드 감지 + 차분 분석
    let cards = detect_cards();
    if cards.is_empty() {
        println!("SD 카드: 미감지");
        println!("  → SD 카드를 꽂으세요.");
    } else {
        for card in &cards {
            println!("SD 카드: {} ({})", card.volume_name, card.device_type);
            println!("  경로: {}", card.volume_path.display());
            println!("  전체: {}개 ({})", card.file_count, human_bytes(card.total_bytes));

            // 차분 분석: 이미 백업된 파일 vs 새 파일
            if !cfg.backup_target.is_empty() {
                if let Some(dcim) = &card.dcim_path {
                    let (new_count, new_bytes, existing) = diff_analysis(dcim, &cfg, &card.device_type);
                    if new_count == 0 {
                        println!("  상태: ✓ 이미 백업 완료 (새 파일 없음)");
                    } else if existing > 0 {
                        println!("  상태: ⚡ 추가 파일 {}개 ({}) — 기존 {}개 skip",
                            new_count, human_bytes(new_bytes), existing);
                    } else {
                        println!("  상태: 📸 새 SD — {}개 백업 필요", new_count);
                    }
                }
            }
        }
    }

    // 최근 이력
    let hist = load_history();
    if let Some(last) = hist.entries.last() {
        println!("\n최근 백업: {} | {} | {}개 파일", last.timestamp, last.device, last.files_copied);
    }
}

/// SD 카드 vs 로컬 백업 차분 분석. (새 파일 수, 새 바이트, 기존 파일 수)
fn diff_analysis(dcim: &Path, cfg: &Config, device_type: &str) -> (usize, u64, usize) {
    let target = PathBuf::from(expand(&cfg.backup_target));
    let files = collect_media_files(dcim);
    let mut new_count = 0usize;
    let mut new_bytes = 0u64;
    let mut existing = 0usize;
    for f in &files {
        let fname = f.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !cfg.include_lrf && fname.to_uppercase().ends_with(".LRF") { continue; }
        let ext = fname.rsplit('.').next().unwrap_or("").to_uppercase();
        if cfg.exclude_extensions.iter().any(|e| e.to_uppercase() == ext) { continue; }

        let date = {
            let d = extract_date(&fname);
            if d == "unknown" {
                // mtime fallback (cmd_run 과 동일 로직)
                fs::metadata(f).ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| {
                        let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
                        Command::new("date").args(["-r", &secs.to_string(), "+%Y-%m-%d"]).output().ok()
                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    })
                    .unwrap_or_else(date_str)
            } else { d }
        };
        let dest = target.join(&date).join(device_type).join(&fname);
        if dest.exists() {
            let src_size = fs::metadata(f).map(|m| m.len()).unwrap_or(0);
            let dst_size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            if src_size == dst_size { existing += 1; continue; }
        }
        new_count += 1;
        new_bytes += fs::metadata(f).map(|m| m.len()).unwrap_or(0);
    }
    (new_count, new_bytes, existing)
}

fn cmd_set_target(path: &str) {
    let mut cfg = load_config();
    cfg.backup_target = path.to_string();
    save_config(&cfg);
    let expanded = expand(path);
    let exists = Path::new(&expanded).exists();
    println!("✓ 백업 대상: {}{}", path, if exists { "" } else { " (⚠ 경로 없음 — 마운트 필요)" });
}

fn cmd_run() {
    if !acquire_lock() {
        eprintln!("✗ 다른 백업이 이미 진행 중입니다.");
        return;
    }
    let cfg = load_config();
    if cfg.backup_target.is_empty() {
        eprintln!("✗ 백업 대상 미설정. `mai run sd-backup set-target <경로>`");
        std::process::exit(1);
    }
    let target_root = PathBuf::from(expand(&cfg.backup_target));
    if !target_root.exists() {
        eprintln!("✗ 백업 대상 경로 없음: {} (NAS 마운트 확인)", target_root.display());
        std::process::exit(1);
    }

    let cards = detect_cards();
    if cards.is_empty() {
        println!("SD 카드 미감지. 카드를 꽂으세요.");
        return;
    }

    let mut history = load_history();

    for card in &cards {
        println!("\n━━━ {} ({}) ━━━", card.volume_name, card.device_type);

        let Some(dcim) = &card.dcim_path else { continue; };

        // 기기별 디렉터리
        // 저장 구조: <백업대상>/<날짜>/<기기명>/파일
        // (기존: <기기>/<날짜> → 변경: <날짜>/<기기>)
        let device_type = &card.device_type;

        // DCIM 하위 파일 수집 + 날짜별 분류
        let files = collect_media_files(dcim);
        if files.is_empty() {
            println!("  미디어 파일 없음.");
            continue;
        }

        // 날짜별 그룹
        let mut by_date: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for f in &files {
            let fname = f.file_name().unwrap_or_default().to_string_lossy().to_string();

            // LRF 제외 옵션
            if !cfg.include_lrf && fname.to_uppercase().ends_with(".LRF") { continue; }
            // 사용자 제외 확장자
            let ext = fname.rsplit('.').next().unwrap_or("").to_uppercase();
            if cfg.exclude_extensions.iter().any(|e| e.to_uppercase() == ext) { continue; }

            let date = extract_date(&fname);
            let date = if date == "unknown" {
                // mtime fallback
                fs::metadata(f).ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| {
                        let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
                        let out = Command::new("date").args(["-r", &secs.to_string(), "+%Y-%m-%d"]).output().ok()?;
                        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
                    })
                    .unwrap_or_else(|| date_str())
            } else { date };

            by_date.entry(date).or_default().push(f.clone());
        }

        // 대상 파일 수 카운트 (skip 포함해서 전체 기준)
        let eligible_files: Vec<PathBuf> = by_date.values().flatten().cloned().collect();
        let files_total = eligible_files.len();
        let bytes_total: u64 = eligible_files.iter()
            .map(|f| fs::metadata(f).map(|m| m.len()).unwrap_or(0)).sum();

        // progress 초기화
        save_progress(&ProgressState {
            running: true,
            device: card.device_type.clone(),
            current_file: String::new(),
            files_done: 0, files_total,
            bytes_done: 0, bytes_total,
            started_at: now_str(),
        });

        let mut total_copied = 0usize;
        let mut total_skipped = 0usize;
        let mut total_bytes = 0u64;
        let mut files_processed = 0usize;
        let mut file_records: Vec<FileRecord> = Vec::new();
        let start_time = std::time::Instant::now();

        // .partial/ 스테이징 — 복사 중단 시 불완전 파일 격리.
        // src → .partial/<날짜>/<기기>/파일 → 크기 검증 → rename → dest/<날짜>/<기기>/파일
        let partial_root = target_root.join(".partial");

        let mut dates: Vec<String> = by_date.keys().cloned().collect();
        dates.sort();
        for date in &dates {
            let files = &by_date[date];
            let dest = target_root.join(date).join(device_type);
            let staging = partial_root.join(date).join(device_type);
            if let Err(e) = fs::create_dir_all(&dest) {
                eprintln!("  ✗ 디렉터리 생성 실패: {}: {}", dest.display(), e);
                continue;
            }
            let _ = fs::create_dir_all(&staging);

            let mut day_copied = 0;
            for src in files {
                let fname_str = src.file_name().unwrap_or_default().to_string_lossy().to_string();
                files_processed += 1;

                save_progress(&ProgressState {
                    running: true,
                    device: card.device_type.clone(),
                    current_file: fname_str.clone(),
                    files_done: files_processed, files_total,
                    bytes_done: total_bytes, bytes_total,
                    started_at: String::new(),
                });

                match copy_staged(src, &dest, &staging) {
                    CopyResult::Skipped => {
                        let size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
                        total_bytes += size;
                        total_skipped += 1;
                    }
                    CopyResult::Copied(size) => {
                        day_copied += 1;
                        total_bytes += size;
                        file_records.push(FileRecord {
                            name: fname_str, size, date: date.clone(),
                        });
                    }
                    CopyResult::SizeMismatch { expected, actual } => {
                        eprintln!("  ✗ 크기 불일치: {} (src={}B staged={}B) — .partial/에 잔류",
                            fname_str, expected, actual);
                    }
                    CopyResult::SourceGone => {
                        eprintln!("  ✗ SD 접근 불가 (제거됨?) — 백업 중단");
                        break;
                    }
                    CopyResult::RsyncFailed => {
                        eprintln!("  ✗ 복사 실패: {}", src.display());
                    }
                    CopyResult::MoveFailed => {
                        eprintln!("  ✗ 이동 실패: {}", fname_str);
                    }
                }
            }
            total_copied += day_copied;
            if day_copied > 0 {
                println!("  ✓ {} → {}개 파일 → {}", date, day_copied, dest.display());
            }

            // 날짜별 staging 디렉터리 정리 (비어있으면 삭제)
            let _ = fs::remove_dir(staging.as_path());
        }
        // .partial/ 하위 빈 디렉터리 정리
        if let Ok(entries) = fs::read_dir(&partial_root) {
            for e in entries.flatten() {
                let _ = fs::remove_dir(e.path()); // 비어있을 때만 성공
            }
        }
        let _ = fs::remove_dir(&partial_root); // 전체 비었으면 삭제

        if total_copied == 0 {
            println!("  (새로운 파일 없음 — 이미 백업됨)");
        } else {
            println!("\n  합계: {}개 파일, {}", total_copied, human_bytes(total_bytes));

            let elapsed = start_time.elapsed().as_secs().max(1);
            let speed = total_bytes / elapsed;
            println!("  속도: {} ({:.0}초)", human_bytes(speed) + "/s", elapsed);

            history.entries.push(HistoryEntry {
                timestamp: now_str(),
                device: card.device_type.clone(),
                volume: card.volume_name.clone(),
                files_copied: total_copied,
                files_skipped: total_skipped,
                bytes_copied: total_bytes,
                duration_secs: elapsed,
                speed_bps: speed,
                target_dir: target_root.to_string_lossy().to_string(),
                files: file_records.clone(),
            });
        }
    }
    save_history(&history);
    clear_progress();

    // NAS 동기화
    for card in &cards {
        sync_to_nas(&cfg, &card.device_type);
    }

    // SD 자동 추출
    if cfg.auto_eject {
        for card in &cards {
            eject_sd(&card.volume_path);
        }
    }

    // macOS 알림
    let eject_msg = if cfg.auto_eject { " → 추출됨" } else { "" };
    let sync_msg = if cfg.sync_enabled { " → NAS 동기화" } else { "" };
    let msg = format!("SD 백업 완료{}{}", sync_msg, eject_msg);
    let _ = Command::new("osascript")
        .args(["-e", &format!("display notification \"{}\" with title \"mac-app-init\"", msg)])
        .output();
    release_lock();
}

fn collect_media_files(dcim: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return; };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() { walk(&path, out); }
            else { out.push(path); }
        }
    }
    walk(dcim, &mut files);
    files.sort();
    files
}

fn cmd_set_sync(path: &str) {
    let mut cfg = load_config();
    cfg.sync_target = path.to_string();
    cfg.sync_enabled = true;
    save_config(&cfg);
    let expanded = expand(path);
    let exists = Path::new(&expanded).exists();
    println!("✓ NAS 동기화 경로: {}{}", path, if exists { "" } else { " (⚠ 경로 없음 — 마운트 확인)" });
}

fn cmd_sync_toggle(toggle: &str) {
    let mut cfg = load_config();
    match toggle.to_lowercase().as_str() {
        "on" | "true" => { cfg.sync_enabled = true; println!("✓ NAS 동기화 켜짐"); }
        "off" | "false" => { cfg.sync_enabled = false; println!("✓ NAS 동기화 꺼짐"); }
        "status" => {
            println!("NAS 동기화: {}", if cfg.sync_enabled { "켜짐" } else { "꺼짐" });
            println!("경로: {}", if cfg.sync_target.is_empty() { "미설정" } else { &cfg.sync_target });
        }
        _ => { eprintln!("사용법: sd-backup sync <on|off|status>"); std::process::exit(1); }
    }
    save_config(&cfg);
}

fn cmd_eject_toggle(toggle: &str) {
    let mut cfg = load_config();
    match toggle.to_lowercase().as_str() {
        "on" | "true" => { cfg.auto_eject = true; println!("✓ 자동 추출 켜짐"); }
        "off" | "false" => { cfg.auto_eject = false; println!("✓ 자동 추출 꺼짐"); }
        "status" => { println!("자동 추출: {}", if cfg.auto_eject { "켜짐" } else { "꺼짐" }); }
        _ => { eprintln!("사용법: sd-backup eject <on|off|status>"); std::process::exit(1); }
    }
    save_config(&cfg);
}

/// 로컬 백업 → NAS 동기화 (rsync)
fn sync_to_nas(cfg: &Config, _device_type: &str) {
    if !cfg.sync_enabled || cfg.sync_target.is_empty() { return; }
    let local = PathBuf::from(expand(&cfg.backup_target));
    let remote = PathBuf::from(expand(&cfg.sync_target));
    if !local.exists() { return; }
    if !remote.exists() {
        eprintln!("  ⚠ NAS 동기화 경로 없음: {} (마운트 확인)", cfg.sync_target);
        return;
    }
    // 전체 백업 디렉터리 rsync (날짜/기기 구조 그대로)
    println!("\n  NAS 동기화: {} → {}", local.display(), remote.display());
    let status = Command::new("rsync")
        .args(["-av", "--progress"])
        .arg(format!("{}/", local.display()))
        .arg(format!("{}/", remote.display()))
        .status();
    match status {
        Ok(s) if s.success() => println!("  ✓ NAS 동기화 완료"),
        _ => eprintln!("  ✗ NAS 동기화 실패"),
    }
}

/// SD 카드 안전 추출
fn eject_sd(volume_path: &Path) {
    println!("\n  SD 추출: {}", volume_path.display());
    let status = Command::new("diskutil")
        .args(["eject", &volume_path.to_string_lossy()])
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("  ✓ SD 카드 안전 추출 완료");
            let _ = Command::new("osascript")
                .args(["-e", "display notification \"SD 카드 안전 추출됨\" with title \"mac-app-init\""])
                .output();
        }
        _ => eprintln!("  ✗ SD 추출 실패"),
    }
}

fn cmd_auto(toggle: &str) {
    let mut cfg = load_config();
    match toggle.to_lowercase().as_str() {
        "on" | "true" => {
            cfg.auto_enabled = true;
            save_config(&cfg);
            install_launchagent();
            println!("✓ SD 자동 백업 켜짐 (30초마다 스캔)");
        }
        "off" | "false" => {
            cfg.auto_enabled = false;
            save_config(&cfg);
            uninstall_launchagent();
            println!("✓ SD 자동 백업 꺼짐");
        }
        "status" => {
            println!("자동 백업: {}", if cfg.auto_enabled { "켜짐" } else { "꺼짐" });
            println!("LaunchAgent: {}", if plist_path().exists() { "✓ 등록" } else { "✗ 미등록" });
        }
        _ => {
            eprintln!("사용법: sd-backup auto <on|off|status>");
            std::process::exit(1);
        }
    }
}

fn install_launchagent() {
    // mac CLI 경유로 실행 → TCC(Documents 접근) 부모 상속
    let mac_bin = Command::new("which").arg("mac").output()
        .ok().and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else { None })
        .unwrap_or_else(|| format!("{}/.cargo/bin/mac", home()));
    let log_dir = format!("{}/Documents/WORK/logs", home());
    let _ = fs::create_dir_all(&log_dir);

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:{home}/.cargo/bin:{home}/.mac-app-init/domains</string>
        <key>HOME</key>
        <string>{home}</string>
    </dict>
    <key>ProgramArguments</key>
    <array>
        <string>{mac_bin}</string>
        <string>run</string>
        <string>sd-backup</string>
        <string>watch</string>
    </array>
    <key>StartInterval</key>
    <integer>30</integer>
    <key>ThrottleInterval</key>
    <integer>30</integer>
    <key>StandardOutPath</key>
    <string>{log}/sd-backup.log</string>
    <key>StandardErrorPath</key>
    <string>{log}/sd-backup.log</string>
</dict>
</plist>
"#, label=LAUNCH_LABEL, mac_bin=mac_bin, home=home(), log=log_dir);

    let path = plist_path();
    if let Some(p) = path.parent() { let _ = fs::create_dir_all(p); }
    let _ = fs::write(&path, plist);
    let _ = Command::new("launchctl").args(["unload", &path.to_string_lossy()]).output();
    let _ = Command::new("launchctl").args(["load", &path.to_string_lossy()]).output();
}

fn uninstall_launchagent() {
    let path = plist_path();
    if path.exists() {
        let _ = Command::new("launchctl").args(["unload", &path.to_string_lossy()]).output();
        let _ = fs::remove_file(&path);
    }
}

/// LaunchAgent 에서 30초마다 호출. SD 감지 시 자동 백업.
fn cmd_watch() {
    let cfg = load_config();
    if !cfg.auto_enabled { return; }
    if cfg.backup_target.is_empty() { return; }

    // lock 먼저 — 이미 다른 백업/watch 실행 중이면 즉시 skip
    if lock_path().exists() { return; }

    // progress 체크
    let prog = load_progress();
    if prog.running { return; }

    // 가벼운 감지 먼저 (DCIM 존재만)
    let light = detect_cards_light();
    if light == "미감지" { return; }

    // 백업 대상 경로
    let target = PathBuf::from(expand(&cfg.backup_target));
    if !target.exists() { return; }

    // 여기서부터 무거운 작업 (timeout 보호된 detect_cards)
    let cards = detect_cards();
    if cards.is_empty() { return; }

    // 차분 분석
    let mut has_new = false;
    for card in &cards {
        if let Some(dcim) = &card.dcim_path {
            let (new_count, _, _) = diff_analysis(dcim, &cfg, &card.device_type);
            if new_count > 0 { has_new = true; }
        }
    }

    if !has_new {
        // 이미 백업 완료된 SD — run 호출 안 함
        println!("[{}] SD 감지 (이미 백업 완료, 새 파일 없음)", now_str());
        return;
    }

    println!("[{}] SD 감지 → 새 파일 발견 → 자동 백업 시작", now_str());
    cmd_run();
}

fn cmd_progress() {
    let prog = load_progress();
    if !prog.running {
        println!("백업 진행 중 아님.");
        return;
    }
    let pct = if prog.bytes_total > 0 {
        (prog.bytes_done as f64 / prog.bytes_total as f64 * 100.0) as u32
    } else { 0 };
    println!("=== 백업 진행 중 ===\n");
    println!("기기  : {}", prog.device);
    println!("파일  : {}/{}", prog.files_done, prog.files_total);
    println!("용량  : {} / {} ({}%)", human_bytes(prog.bytes_done), human_bytes(prog.bytes_total), pct);
    println!("현재  : {}", prog.current_file);
    if !prog.started_at.is_empty() {
        println!("시작  : {}", prog.started_at);
    }
}

fn cmd_devices() {
    let hist = load_history();
    let mut devices: HashMap<String, usize> = HashMap::new();
    for e in &hist.entries {
        *devices.entry(e.device.clone()).or_default() += 1;
    }
    if devices.is_empty() {
        println!("백업 이력 없음. SD 카드를 꽂고 `mai run sd-backup run` 실행.");
        return;
    }
    println!("{:<25} {}", "DEVICE", "BACKUPS");
    println!("{}", "─".repeat(40));
    for (dev, count) in &devices {
        println!("{:<25} {}회", dev, count);
    }
}

fn cmd_history() {
    let hist = load_history();
    if hist.entries.is_empty() {
        println!("백업 이력 없음.");
        return;
    }
    for (i, e) in hist.entries.iter().rev().enumerate() {
        if i >= 10 { break; }
        println!("━━━ {} ━━━", e.timestamp);
        println!("  기기    : {}", e.device);
        println!("  볼륨    : {}", e.volume);
        println!("  복사    : {}개 (skip {}개)", e.files_copied, e.files_skipped);
        println!("  용량    : {}", human_bytes(e.bytes_copied));
        if e.duration_secs > 0 {
            println!("  시간    : {}초", e.duration_secs);
            println!("  속도    : {}/s", human_bytes(e.speed_bps));
        }
        println!("  저장 위치: {}", e.target_dir);
        if !e.files.is_empty() {
            println!("  파일:");
            for f in &e.files {
                println!("    {} ({})", f.name, human_bytes(f.size));
            }
        }
        println!();
    }
}

fn print_tui_spec() {
    let cfg = load_config();
    let hist = load_history();

    // tui-spec 은 가볍게 — SD 파일 스캔 안 함 (detect_cards/diff_analysis 호출 금지).
    // 5초마다 refresh 되므로 무거운 작업은 status/run 에서만.
    let card_info = detect_cards_light();

    let target_status = if cfg.backup_target.is_empty() { "미설정" }
        else if Path::new(&expand(&cfg.backup_target)).exists() { "✓ 접근 가능" }
        else { "✗ 경로 없음" };

    let last_backup = hist.entries.last()
        .map(|e| format!("{} | {} | {}개", e.timestamp, e.device, e.files_copied))
        .unwrap_or_else(|| "없음".into());

    // 진행 상태
    let prog = load_progress();
    let prog_info = if prog.running {
        let pct = if prog.bytes_total > 0 { (prog.bytes_done as f64 / prog.bytes_total as f64 * 100.0) as u32 } else { 0 };
        format!("▶ 백업 중 ({}/{} 파일, {}%) — {}", prog.files_done, prog.files_total, pct, prog.current_file)
    } else { "대기 중".into() };

    let sync_status = if cfg.sync_target.is_empty() { "미설정" }
        else if !cfg.sync_enabled { "꺼짐" }
        else if Path::new(&expand(&cfg.sync_target)).exists() { "✓ 켜짐 (접근 가능)" }
        else { "⚠ 켜짐 (경로 없음 — 마운트 확인)" };

    // .partial/ 잔류 파일 체크 (가벼운 디렉터리 존재 여부만)
    let partial_dir = PathBuf::from(expand(&cfg.backup_target)).join(".partial");
    let partial_count = count_partial_files(&partial_dir);

    let usage_active = cfg.auto_enabled;
    let usage_summary = if cfg.auto_enabled { "자동백업 켜짐".to_string() } else { "꺼짐".to_string() };

    let mut status_items = vec![
        tui_spec::kv_item("SD 카드", &card_info,
            if card_info == "미감지" { "warn" } else { "ok" }),
        tui_spec::kv_item("진행 상태", &prog_info,
            if prog.running { "ok" } else { "warn" }),
        tui_spec::kv_item("최근 백업", &last_backup, "ok"),
    ];
    if partial_count > 0 {
        status_items.push(tui_spec::kv_item(
            "미완료 파일",
            &format!("⚠ {}개 (재연결 시 자동 이어받기)", partial_count),
            "warn",
        ));
    }

    TuiSpec::new("sd-backup")
        .refresh(5)
        .usage(usage_active, &usage_summary)
        .kv("상태", status_items)
        .kv("설정", vec![
            tui_spec::kv_item("로컬 백업 경로",
                &format!("{} ({})", cfg.backup_target, target_status),
                if cfg.backup_target.is_empty() { "error" } else { "ok" }),
            tui_spec::kv_item("NAS 동기화",
                &format!("{} — {}", sync_status,
                    if cfg.sync_target.is_empty() { "미설정".into() } else { cfg.sync_target.clone() }),
                if cfg.sync_enabled && !cfg.sync_target.is_empty() { "ok" } else { "warn" }),
            tui_spec::kv_item("자동 백업",
                if cfg.auto_enabled { "✓ 켜짐 (SD 꽂으면 30초 내 시작)" } else { "꺼짐" },
                if cfg.auto_enabled { "ok" } else { "warn" }),
            tui_spec::kv_item("자동 추출",
                if cfg.auto_eject { "✓ 백업 완료 후 SD 자동 추출" } else { "꺼짐 (수동 추출)" },
                if cfg.auto_eject { "ok" } else { "warn" }),
            tui_spec::kv_item("LRF 포함",
                if cfg.include_lrf { "예 (저해상도 프리뷰)" } else { "아니오 (MP4만)" },
                "ok"),
        ])
        .buttons()
        .buttons_custom("설정 토글", vec![
            serde_json::json!({
                "label": if cfg.auto_enabled { "자동백업 OFF" } else { "자동백업 ON" },
                "command": if cfg.auto_enabled { "auto off" } else { "auto on" },
                "key": "a"
            }),
            serde_json::json!({
                "label": if cfg.sync_enabled { "NAS동기화 OFF" } else { "NAS동기화 ON" },
                "command": if cfg.sync_enabled { "sync off" } else { "sync on" },
                "key": "y"
            }),
            serde_json::json!({
                "label": if cfg.auto_eject { "자동추출 OFF" } else { "자동추출 ON" },
                "command": if cfg.auto_eject { "eject off" } else { "eject on" },
                "key": "e"
            }),
        ])
        .text("안내", "  SD 꽂음 → 로컬 백업 (60MB/s) → NAS 동기화 (LAN 권장) → SD 추출\n\n  기기별·날짜별 자동 분류:\n    <로컬>/<기기명>/<YYYY-MM-DD>/파일들\n\n  설정:\n    mai run sd-backup set-target <로컬 경로>\n    mai run sd-backup set-sync <NAS 경로>\n    mai run sd-backup auto on\n    mai run sd-backup eject on")
        .print();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let p = dir.join(name);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    #[test]
    fn copy_staged_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dest_dir = tmp.path().join("dest");
        let staging_dir = tmp.path().join("staging");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        let src = make_file(&src_dir, "DJI_0001.MP4", b"fake video data 12345");

        let result = copy_staged(&src, &dest_dir, &staging_dir);
        assert_eq!(result, CopyResult::Copied(21));
        // 정식 위치에 존재
        assert!(dest_dir.join("DJI_0001.MP4").exists());
        // staging에는 없음 (rename으로 이동됨)
        assert!(!staging_dir.join("DJI_0001.MP4").exists());
    }

    #[test]
    fn copy_staged_skip_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dest_dir = tmp.path().join("dest");
        let staging_dir = tmp.path().join("staging");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        let content = b"same content";
        make_file(&src_dir, "IMG_001.CR3", content);
        make_file(&dest_dir, "IMG_001.CR3", content); // 이미 있음

        let result = copy_staged(&src_dir.join("IMG_001.CR3"), &dest_dir, &staging_dir);
        assert_eq!(result, CopyResult::Skipped);
    }

    #[test]
    fn copy_staged_overwrite_different_size() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dest_dir = tmp.path().join("dest");
        let staging_dir = tmp.path().join("staging");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        make_file(&src_dir, "VID.MP4", b"new longer content here");
        make_file(&dest_dir, "VID.MP4", b"old short"); // 크기 다름

        let result = copy_staged(&src_dir.join("VID.MP4"), &dest_dir, &staging_dir);
        assert_eq!(result, CopyResult::Copied(23));
        // 새 파일로 교체됨
        let final_size = fs::metadata(dest_dir.join("VID.MP4")).unwrap().len();
        assert_eq!(final_size, 23);
    }

    #[test]
    fn copy_staged_source_gone() {
        let tmp = tempfile::tempdir().unwrap();
        let dest_dir = tmp.path().join("dest");
        let staging_dir = tmp.path().join("staging");
        fs::create_dir_all(&dest_dir).unwrap();

        let fake_src = tmp.path().join("nonexistent.MP4");
        let result = copy_staged(&fake_src, &dest_dir, &staging_dir);
        assert_eq!(result, CopyResult::SourceGone);
    }

    #[test]
    fn count_partial_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(count_partial_files(tmp.path()), 0);
    }

    #[test]
    fn count_partial_nonexistent() {
        let fake = PathBuf::from("/tmp/nonexistent_partial_test_dir");
        assert_eq!(count_partial_files(&fake), 0);
    }

    #[test]
    fn count_partial_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        let partial = tmp.path().join(".partial");
        let sub = partial.join("2025-01-01").join("DJI");
        fs::create_dir_all(&sub).unwrap();
        make_file(&sub, "DJI_0001.MP4", b"partial1");
        make_file(&sub, "DJI_0002.MP4", b"partial2");

        let sub2 = partial.join("2025-01-02").join("Canon");
        fs::create_dir_all(&sub2).unwrap();
        make_file(&sub2, "IMG_001.CR3", b"partial3");

        assert_eq!(count_partial_files(&partial), 3);
    }

    #[test]
    fn copy_staged_creates_staging_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dest_dir = tmp.path().join("dest");
        let staging_dir = tmp.path().join("deep/nested/staging");
        fs::create_dir_all(&src_dir).unwrap();
        // dest_dir과 staging_dir은 copy_staged가 생성

        let src = make_file(&src_dir, "test.dat", b"data");
        let result = copy_staged(&src, &dest_dir, &staging_dir);
        assert_eq!(result, CopyResult::Copied(4));
        assert!(dest_dir.join("test.dat").exists());
    }

    #[test]
    fn copy_staged_multiple_sequential() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dest_dir = tmp.path().join("dest");
        let staging_dir = tmp.path().join("staging");
        fs::create_dir_all(&src_dir).unwrap();

        // 3개 파일 순차 복사
        for i in 0..3 {
            let name = format!("file_{}.dat", i);
            let content = format!("content {}", i);
            make_file(&src_dir, &name, content.as_bytes());

            let result = copy_staged(&src_dir.join(&name), &dest_dir, &staging_dir);
            assert_eq!(result, CopyResult::Copied(content.len() as u64));
        }
        // 전부 정식 위치에 존재
        assert!(dest_dir.join("file_0.dat").exists());
        assert!(dest_dir.join("file_1.dat").exists());
        assert!(dest_dir.join("file_2.dat").exists());
    }

    #[test]
    fn copy_staged_then_skip_on_retry() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dest_dir = tmp.path().join("dest");
        let staging_dir = tmp.path().join("staging");
        fs::create_dir_all(&src_dir).unwrap();

        let src = make_file(&src_dir, "video.mp4", b"video data");

        // 첫 복사
        let r1 = copy_staged(&src, &dest_dir, &staging_dir);
        assert_eq!(r1, CopyResult::Copied(10));

        // 재시도 → skip
        let r2 = copy_staged(&src, &dest_dir, &staging_dir);
        assert_eq!(r2, CopyResult::Skipped);
    }
}
