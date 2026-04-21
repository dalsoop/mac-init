use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{count_files, dir_size, home};
use crate::common;

pub const SD_PLIST: &str = "com.mac-host.sd-backup";

const SKIP_VOLUMES: &[&str] = &[
    "Macintosh HD",
    "Macintosh HD - Data",
    "Recovery",
    "Preboot",
    "VM",
    "Update",
    "proxmox",
    "synology",
    "truenas",
];

const VIDEO_DIRS: &[&str] = &["PRIVATE", "AVCHD", "CLIP"];

const RSYNC_EXCLUDE: &[&str] = &[
    "DCIM",
    "PRIVATE",
    "AVCHD",
    "CLIP",
    "System Volume Information",
    ".Spotlight-V100",
    ".fseventsd",
];

// === Data functions ===

fn detect_sd_volumes() -> Vec<PathBuf> {
    let mut volumes = Vec::new();
    let volumes_dir = Path::new("/Volumes");
    if !volumes_dir.is_dir() {
        return volumes;
    }

    if let Ok(entries) = fs::read_dir(volumes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if SKIP_VOLUMES.iter().any(|s| name.starts_with(s)) {
                continue;
            }
            // Check if removable media via diskutil
            let (ok, stdout, _) = common::run_cmd("diskutil", &["info", &path.to_string_lossy()]);
            if ok
                && (stdout.contains("Removable Media: Removable")
                    || stdout.contains("Protocol: USB")
                    || stdout.contains("Protocol: Secure Digital"))
            {
                volumes.push(path);
            }
        }
    }
    volumes
}

fn log_path() -> PathBuf {
    PathBuf::from(home()).join("문서/시스템/로그/sd-backup.log")
}

fn backup_base() -> PathBuf {
    PathBuf::from(home()).join("문서/미디어/사진/SD백업")
}

fn synology_mirror() -> PathBuf {
    PathBuf::from("/Volumes/synology/백업/미러리스")
}

// === Actions ===

pub fn sd_run() {
    let volumes = detect_sd_volumes();
    if volumes.is_empty() {
        println!("[sd] SD 카드가 감지되지 않았습니다.");
        return;
    }

    let date = chrono_now();
    let log = log_path();
    common::ensure_dir(log.parent().unwrap());
    common::ensure_dir(&backup_base());

    for sd in &volumes {
        let vol_name = sd.file_name().unwrap().to_string_lossy().to_string();
        let backup_dir = backup_base().join(format!("{}_{}", date, vol_name));

        append_log(
            &log,
            &format!("[{}] SD 카드 감지: {} ({})", date, vol_name, sd.display()),
        );

        // DCIM
        let dcim = sd.join("DCIM");
        if dcim.is_dir() {
            let dest = backup_dir.join("DCIM");
            fs::create_dir_all(&dest).ok();
            let (ok, stdout, _) = common::run_cmd(
                "rsync",
                &[
                    "-av",
                    &format!("{}/", dcim.display()),
                    &format!("{}/", dest.display()),
                ],
            );
            if ok {
                let count = count_files(&dest.to_string_lossy());
                append_log(
                    &log,
                    &format!("[{}] DCIM 백업 완료: {}개 파일", date, count),
                );
                println!("  ✓ DCIM: {}개 파일 → {}", count, dest.display());
            }
            // Synology mirror
            let mirror = synology_mirror();
            if mirror.is_dir() {
                let sync_dir = mirror.join(format!("SD_{}_{}", date, vol_name));
                fs::create_dir_all(&sync_dir).ok();
                common::run_cmd(
                    "rsync",
                    &[
                        "-av",
                        &format!("{}/", dcim.display()),
                        &format!("{}/", sync_dir.display()),
                    ],
                );
                append_log(
                    &log,
                    &format!("[{}] Synology 백업 완료: {}", date, sync_dir.display()),
                );
            }
        }

        // Video dirs
        for vdir in VIDEO_DIRS {
            let src = sd.join(vdir);
            if src.is_dir() {
                let dest = backup_dir.join(vdir);
                fs::create_dir_all(&dest).ok();
                common::run_cmd(
                    "rsync",
                    &[
                        "-av",
                        &format!("{}/", src.display()),
                        &format!("{}/", dest.display()),
                    ],
                );
                append_log(&log, &format!("[{}] {} 백업 완료", date, vdir));
                println!("  ✓ {}", vdir);
            }
        }

        // Misc files
        let misc_dest = backup_dir.join("기타");
        fs::create_dir_all(&misc_dest).ok();
        let mut rsync_args = vec!["-av".to_string()];
        for exc in RSYNC_EXCLUDE {
            rsync_args.push(format!("--exclude={}", exc));
        }
        rsync_args.push(format!("{}/", sd.display()));
        rsync_args.push(format!("{}/", misc_dest.display()));
        let args_ref: Vec<&str> = rsync_args.iter().map(|s| s.as_str()).collect();
        common::run_cmd("rsync", &args_ref);

        // macOS notification
        let _ = Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "display notification \"{}  → {}\" with title \"SD 카드 백업 완료\" sound name \"Glass\"",
                    vol_name, backup_dir.display()
                ),
            ])
            .output();

        append_log(&log, &format!("[{}] 백업 완료: {}", date, vol_name));
        println!("  ✓ {} 백업 완료 → {}", vol_name, backup_dir.display());
    }
}

pub fn sd_status() {
    let h = home();
    let plist = format!("{h}/Library/LaunchAgents/{SD_PLIST}.plist");
    let enabled = Path::new(&plist).exists();

    println!("=== SD 자동 백업 ===\n");
    println!(
        "[자동 백업] {}",
        if enabled {
            "✓ 활성화"
        } else {
            "✗ 비활성화"
        }
    );

    let backup_dir = backup_base();
    if backup_dir.exists() {
        let count = count_files(&backup_dir.to_string_lossy());
        let size = dir_size(&backup_dir.to_string_lossy());
        println!("[백업 이력] {count}개 세션, {size}");

        if let Ok(entries) = fs::read_dir(&backup_dir) {
            let mut names: Vec<String> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            names.sort();
            for name in names.iter().rev().take(5) {
                println!("  {name}");
            }
        }
    }

    let log = log_path();
    if log.exists() {
        let (_, tail) = common::run_cmd_quiet("tail", &["-3", &log.to_string_lossy()]);
        if !tail.trim().is_empty() {
            println!("\n[최근 로그]");
            for line in tail.lines() {
                println!("  {}", line.trim());
            }
        }
    }
}

pub fn sd_enable() {
    let h = home();
    let plist_path = format!("{h}/Library/LaunchAgents/{SD_PLIST}.plist");

    if Path::new(&plist_path).exists() {
        println!("[sd] 이미 활성화됨");
        return;
    }

    // CLI 바이너리 경로
    let bin = which_mac_host_commands();

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>files</string>
        <string>sd-run</string>
    </array>
    <key>WatchPaths</key>
    <array>
        <string>/Volumes</string>
    </array>
    <key>StandardOutPath</key>
    <string>{home}/문서/시스템/로그/sd-backup.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/문서/시스템/로그/sd-backup.log</string>
</dict>
</plist>"#,
        label = SD_PLIST,
        bin = bin,
        home = h
    );

    common::ensure_dir(Path::new(&format!("{h}/문서/시스템/로그")));
    common::ensure_dir(&backup_base());
    fs::write(&plist_path, plist).expect("LaunchAgent 생성 실패");

    let _ = Command::new("launchctl")
        .args(["load", &plist_path])
        .status();
    println!("[sd] 자동 백업 활성화 완료");
    println!("  SD 카드 삽입 시 자동 백업 (mac-host-commands files sd-run)");
}

pub fn sd_disable() {
    let h = home();
    let plist_path = format!("{h}/Library/LaunchAgents/{SD_PLIST}.plist");

    if !Path::new(&plist_path).exists() {
        println!("[sd] 이미 비활성화됨");
        return;
    }

    let _ = Command::new("launchctl")
        .args(["unload", &plist_path])
        .status();
    fs::remove_file(&plist_path).ok();
    println!("[sd] 자동 백업 비활성화 완료");
}

// === Helpers ===

fn append_log(path: &Path, msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", msg);
    }
}

fn chrono_now() -> String {
    let output = Command::new("date").args(["+%y%m%d_%H%M"]).output();
    output
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn which_mac_host_commands() -> String {
    let (ok, stdout) = common::run_cmd_quiet("which", &["mac-host-commands"]);
    if ok {
        stdout.trim().to_string()
    } else {
        format!("{}/.cargo/bin/mac-host-commands", home())
    }
}
