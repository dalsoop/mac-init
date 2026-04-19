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
    /// 백업 대상 경로 설정
    SetTarget { path: String },
    /// 기기 프로필 목록
    Devices,
    /// 백업 이력 조회
    History,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Run => cmd_run(),
        Commands::SetTarget { path } => cmd_set_target(&path),
        Commands::Devices => cmd_devices(),
        Commands::History => cmd_history(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

// === 설정 ===

fn home() -> String { std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()) }
fn config_path() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/sd-backup.json") }
fn history_path() -> PathBuf { PathBuf::from(home()).join(".mac-app-init/sd-backup-history.json") }

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    /// 백업 대상 루트 경로 (예: ~/NAS/synology/works/백업)
    #[serde(default)]
    backup_target: String,
    /// LRF(저해상도 프리뷰) 파일 포함 여부
    #[serde(default)]
    include_lrf: bool,
    /// 제외 확장자
    #[serde(default)]
    exclude_extensions: Vec<String>,
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
    bytes_copied: u64,
    target_dir: String,
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

fn expand(p: &str) -> String {
    if p.starts_with('~') { p.replacen('~', &home(), 1) } else { p.to_string() }
}

fn now_str() -> String {
    Command::new("date").args(["+%Y-%m-%d %H:%M:%S"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn date_str() -> String {
    Command::new("date").args(["+%Y-%m-%d"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

// === 기기 감지 ===

#[derive(Debug)]
struct DetectedCard {
    volume_path: PathBuf,
    volume_name: String,
    device_type: String,        // "DJI-Action-Pro-5", "Generic-Camera", ...
    dcim_path: Option<PathBuf>,
    file_count: usize,
    total_bytes: u64,
}

/// /Volumes/ 스캔해서 DCIM 있는 볼륨 감지 + 기기 판별
fn detect_cards() -> Vec<DetectedCard> {
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
    // DJI: DCIM/DJI_NNN/ + MISC/AC*.db
    if vol.join("DCIM/DJI_001").exists() || vol.join("MISC").exists() {
        // DJI 모델 구분: MISC/AC004.db → Action 시리즈
        if vol.join("MISC/AC004.db").exists() {
            return "DJI-Action-Pro-5".into();
        }
        if vol.join("MISC/AC003.db").exists() {
            return "DJI-Action-4".into();
        }
        return "DJI-Camera".into();
    }
    // GoPro: DCIM/NNNGOPR/
    let dcim = vol.join("DCIM");
    if let Ok(it) = fs::read_dir(&dcim) {
        for e in it.filter_map(|x| x.ok()) {
            let n = e.file_name().to_string_lossy().to_string();
            if n.contains("GOPR") || n.contains("HERO") { return "GoPro".into(); }
        }
    }
    // Sony: DCIM/NNNMSDCF/ or PRIVATE/M4ROOT/
    if vol.join("PRIVATE/M4ROOT").exists() { return "Sony-Camera".into(); }
    // Canon: DCIM/NNNCANON/
    if let Ok(it) = fs::read_dir(&dcim) {
        for e in it.filter_map(|x| x.ok()) {
            let n = e.file_name().to_string_lossy().to_string().to_uppercase();
            if n.contains("CANON") { return "Canon-Camera".into(); }
            if n.contains("NIKON") { return "Nikon-Camera".into(); }
        }
    }
    "Generic-Camera".into()
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
        println!("⚠ 백업 대상 미설정. `mac run sd-backup set-target <경로>` 필요");
    } else {
        let expanded = expand(&cfg.backup_target);
        let exists = Path::new(&expanded).exists();
        println!("백업 대상: {} {}", cfg.backup_target, if exists { "✓" } else { "✗ 경로 없음 (마운트 확인)" });
    }
    println!("LRF 포함: {}", if cfg.include_lrf { "예" } else { "아니오" });
    println!();

    // SD 카드 감지
    let cards = detect_cards();
    if cards.is_empty() {
        println!("SD 카드: 미감지");
        println!("  → SD 카드를 꽂으세요.");
    } else {
        for card in &cards {
            println!("SD 카드: {} ({})", card.volume_name, card.device_type);
            println!("  경로: {}", card.volume_path.display());
            println!("  파일: {}개 ({})", card.file_count, human_bytes(card.total_bytes));
        }
    }

    // 최근 이력
    let hist = load_history();
    if let Some(last) = hist.entries.last() {
        println!("\n최근 백업: {} | {} | {}개 파일", last.timestamp, last.device, last.files_copied);
    }
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
    let cfg = load_config();
    if cfg.backup_target.is_empty() {
        eprintln!("✗ 백업 대상 미설정. `mac run sd-backup set-target <경로>`");
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
        let device_dir = target_root.join(&card.device_type);

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

        let mut total_copied = 0usize;
        let mut total_bytes = 0u64;

        let mut dates: Vec<String> = by_date.keys().cloned().collect();
        dates.sort();
        for date in &dates {
            let files = &by_date[date];
            let dest = device_dir.join(date);
            if let Err(e) = fs::create_dir_all(&dest) {
                eprintln!("  ✗ 디렉터리 생성 실패: {}: {}", dest.display(), e);
                continue;
            }

            let mut day_copied = 0;
            for src in files {
                let fname = src.file_name().unwrap_or_default();
                let dst = dest.join(fname);
                if dst.exists() {
                    // 이미 있으면 크기 비교 — 같으면 skip
                    let src_size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
                    let dst_size = fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
                    if src_size == dst_size { continue; }
                }
                // rsync 로 복사 (progress + resume 가능)
                let status = Command::new("rsync")
                    .args(["-a", "--progress"])
                    .arg(src)
                    .arg(&dest)
                    .status();
                match status {
                    Ok(s) if s.success() => {
                        day_copied += 1;
                        total_bytes += fs::metadata(src).map(|m| m.len()).unwrap_or(0);
                    }
                    _ => eprintln!("  ✗ 복사 실패: {}", src.display()),
                }
            }
            total_copied += day_copied;
            if day_copied > 0 {
                println!("  ✓ {} → {}개 파일 → {}", date, day_copied, dest.display());
            }
        }

        if total_copied == 0 {
            println!("  (새로운 파일 없음 — 이미 백업됨)");
        } else {
            println!("\n  합계: {}개 파일, {}", total_copied, human_bytes(total_bytes));

            // 이력 기록
            history.entries.push(HistoryEntry {
                timestamp: now_str(),
                device: card.device_type.clone(),
                volume: card.volume_name.clone(),
                files_copied: total_copied,
                bytes_copied: total_bytes,
                target_dir: device_dir.to_string_lossy().to_string(),
            });
        }
    }
    save_history(&history);
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

fn cmd_devices() {
    let hist = load_history();
    let mut devices: HashMap<String, usize> = HashMap::new();
    for e in &hist.entries {
        *devices.entry(e.device.clone()).or_default() += 1;
    }
    if devices.is_empty() {
        println!("백업 이력 없음. SD 카드를 꽂고 `mac run sd-backup run` 실행.");
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
    println!("{:<20} {:<22} {:<8} {}", "TIMESTAMP", "DEVICE", "FILES", "SIZE");
    println!("{}", "─".repeat(65));
    for e in hist.entries.iter().rev().take(20) {
        println!("{:<20} {:<22} {:<8} {}", e.timestamp, e.device, e.files_copied, human_bytes(e.bytes_copied));
    }
}

fn print_tui_spec() {
    let cfg = load_config();
    let cards = detect_cards();
    let hist = load_history();

    let card_info = if cards.is_empty() {
        "미감지".into()
    } else {
        cards.iter().map(|c| format!("{} ({}개, {})", c.device_type, c.file_count, human_bytes(c.total_bytes)))
            .collect::<Vec<_>>().join(", ")
    };

    let target_status = if cfg.backup_target.is_empty() { "미설정" }
        else if Path::new(&expand(&cfg.backup_target)).exists() { "✓ 접근 가능" }
        else { "✗ 경로 없음" };

    let last_backup = hist.entries.last()
        .map(|e| format!("{} | {} | {}개", e.timestamp, e.device, e.files_copied))
        .unwrap_or_else(|| "없음".into());

    let spec = serde_json::json!({
        "tab": { "label_ko": "SD 미디어 백업", "label": "SD Backup", "icon": "📸" },
        "group": "auto",
        "sections": [
            {
                "kind": "key-value",
                "title": "상태",
                "items": [
                    { "key": "SD 카드", "value": card_info, "status": if cards.is_empty() { "warn" } else { "ok" } },
                    { "key": "백업 대상", "value": format!("{} ({})", cfg.backup_target, target_status),
                      "status": if cfg.backup_target.is_empty() { "error" } else { "ok" } },
                    { "key": "LRF 포함", "value": if cfg.include_lrf { "예" } else { "아니오" }, "status": "ok" },
                    { "key": "최근 백업", "value": last_backup, "status": "ok" },
                ]
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status", "command": "status", "key": "s" },
                    { "label": "Run (백업 실행)", "command": "run", "key": "r" },
                    { "label": "History (이력)", "command": "history", "key": "h" },
                    { "label": "Devices (기기 목록)", "command": "devices", "key": "v" },
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "  mac run sd-backup set-target ~/NAS/synology/works/백업\n  mac run sd-backup run\n\n  기기별·날짜별 자동 분류:\n    <백업대상>/<기기명>/<YYYY-MM-DD>/파일들"
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
