use std::path::Path;
use std::process::Command;
use std::fs;

use crate::common;
use super::home;
use super::rename::classify_file;
use super::rename::format_filename;

pub fn organize() {
    let h = home();
    let downloads = format!("{h}/Downloads");

    println!("[files] Downloads 정리 중...\n");

    let entries: Vec<_> = fs::read_dir(&downloads)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !name.starts_with('.') && name != ".localized"
        })
        .collect();

    if entries.is_empty() {
        println!("  정리할 파일 없음");
        return;
    }

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            let dest = format!("{h}/임시/{name}");
            if !Path::new(&dest).exists() {
                fs::rename(&path, &dest).ok();
                println!("  📁 {name} → 임시/");
            }
            continue;
        }

        let category = classify_file(&path);
        let new_name = format_filename(&path);
        let dest_dir = format!("{h}/{category}");

        common::ensure_dir(Path::new(&dest_dir));

        let dest = format!("{dest_dir}/{new_name}");
        if !Path::new(&dest).exists() {
            fs::rename(&path, &dest).ok();
            println!("  {name} → {category}/{new_name}");
        } else {
            println!("  ⚠ {name} — 이미 존재 (스킵)");
        }
    }

    println!("\n[files] 정리 완료");
}

pub fn cleanup_temp() {
    let h = home();
    let temp_dir = format!("{h}/임시");

    println!("[files] 임시 폴더 정리 (30일 이상)...\n");

    let output = Command::new("find")
        .args([&temp_dir, "-maxdepth", "1", "-mtime", "+30", "-not", "-name", ".DS_Store"])
        .output()
        .expect("find 실행 실패");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let old_files: Vec<&str> = stdout.lines().filter(|l| !l.is_empty() && *l != temp_dir).collect();

    if old_files.is_empty() {
        println!("  30일 이상 된 파일 없음");
        return;
    }

    let archive_dir = format!("{h}/아카이브/임시정리");
    common::ensure_dir(Path::new(&archive_dir));

    for f in &old_files {
        let name = Path::new(f).file_name().unwrap().to_string_lossy().to_string();
        let dest = format!("{archive_dir}/{name}");
        fs::rename(f, &dest).ok();
        println!("  {name} → 아카이브/임시정리/");
    }

    println!("\n[files] {}개 파일 아카이브로 이동", old_files.len());
}

pub fn setup_auto() {
    let h = home();
    let plist_path = format!("{h}/Library/LaunchAgents/com.mac-host.file-organizer.plist");

    if Path::new(&plist_path).exists() {
        println!("[files] 자동 정리 이미 설정됨");
        return;
    }

    // CLI 바이너리를 직접 호출 (shell 스크립트 불필요)
    let bin = which_bin();

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mac-host.file-organizer</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-c</string>
        <string>{bin} files organize; {bin} files cleanup-temp</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>9</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>{home}/문서/시스템/로그/file-organizer.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/문서/시스템/로그/file-organizer.log</string>
</dict>
</plist>"#, bin = bin, home = h);

    common::ensure_dir(Path::new(&format!("{h}/문서/시스템/로그")));
    fs::write(&plist_path, plist).expect("LaunchAgent 생성 실패");
    let _ = Command::new("launchctl").args(["load", &plist_path]).status();

    println!("[files] 자동 정리 설정 완료");
    println!("  매일 09:00 실행 (mai run files organize + mai run files cleanup-temp)");
}

fn which_bin() -> String {
    common::manager_bin().display().to_string()
}

pub fn disable_auto() {
    let h = home();
    let plist_path = format!("{h}/Library/LaunchAgents/com.mac-host.file-organizer.plist");

    if !Path::new(&plist_path).exists() {
        println!("[files] 자동 정리가 설정되어 있지 않습니다.");
        return;
    }

    let _ = Command::new("launchctl").args(["unload", &plist_path]).status();
    fs::remove_file(&plist_path).ok();
    println!("[files] 자동 정리 비활성화 완료");
}
