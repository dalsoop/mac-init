use std::path::Path;
use std::process::Command;
use std::fs;

use crate::common;
use super::{home, count_files, dir_size};

pub const SD_PLIST: &str = "com.mac-host.sd-backup.plist";
const SD_SCRIPT: &str = "문서/시스템/bin/sd-backup.sh";

pub fn sd_status() {
    let h = home();
    let plist = format!("{h}/Library/LaunchAgents/{SD_PLIST}");
    let enabled = Path::new(&plist).exists();

    println!("=== SD 자동 백업 ===\n");
    println!("[자동 백업] {}", if enabled { "✓ 활성화" } else { "✗ 비활성화" });

    let backup_dir = format!("{h}/미디어/사진/SD백업");
    if Path::new(&backup_dir).exists() {
        let count = count_files(&backup_dir);
        let size = dir_size(&backup_dir);
        println!("[백업 이력] {count}개 세션, {size}");

        let entries: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        let mut names: Vec<String> = entries.iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        for name in names.iter().rev().take(5) {
            println!("  {name}");
        }
    }

    let log = format!("{h}/시스템/로그/sd-backup.log");
    if Path::new(&log).exists() {
        let (_, tail) = common::run_cmd_quiet("tail", &["-3", &log]);
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
    let plist_path = format!("{h}/Library/LaunchAgents/{SD_PLIST}");

    if Path::new(&plist_path).exists() {
        println!("[sd] 이미 활성화됨");
        return;
    }

    let script_path = format!("{h}/{SD_SCRIPT}");
    if !Path::new(&script_path).exists() {
        eprintln!("[sd] 백업 스크립트가 없습니다: {script_path}");
        std::process::exit(1);
    }

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mac-host.sd-backup</string>
    <key>ProgramArguments</key>
    <array>
        <string>{script}</string>
    </array>
    <key>WatchPaths</key>
    <array>
        <string>/Volumes</string>
    </array>
    <key>StandardOutPath</key>
    <string>{home}/시스템/로그/sd-backup.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/시스템/로그/sd-backup.log</string>
</dict>
</plist>"#, script = script_path, home = h);

    common::ensure_dir(Path::new(&format!("{h}/시스템/로그")));
    common::ensure_dir(Path::new(&format!("{h}/미디어/사진/SD백업")));
    fs::write(&plist_path, plist).expect("LaunchAgent 생성 실패");

    let _ = Command::new("launchctl").args(["load", &plist_path]).status();
    println!("[sd] 자동 백업 활성화 완료");
    println!("  SD 카드 삽입 시 자동 백업");
    println!("  → ~/미디어/사진/SD백업/");
    println!("  → /Volumes/synology/백업/미러리스/ (마운트 시)");
}

pub fn sd_disable() {
    let h = home();
    let plist_path = format!("{h}/Library/LaunchAgents/{SD_PLIST}");

    if !Path::new(&plist_path).exists() {
        println!("[sd] 이미 비활성화됨");
        return;
    }

    let _ = Command::new("launchctl").args(["unload", &plist_path]).status();
    fs::remove_file(&plist_path).ok();
    println!("[sd] 자동 백업 비활성화 완료");
}

pub fn sd_run() {
    let h = home();
    let script_path = format!("{h}/{SD_SCRIPT}");

    if !Path::new(&script_path).exists() {
        eprintln!("[sd] 백업 스크립트가 없습니다.");
        std::process::exit(1);
    }

    println!("[sd] 수동 백업 실행 중...");
    let _ = Command::new("bash").args([&script_path]).status();
}
