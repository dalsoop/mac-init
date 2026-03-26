use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

use crate::common;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/jeonghan".to_string())
}

// 파일 분류 규칙
fn classify_file(path: &Path) -> &'static str {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // 이미지
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" | "tiff" => "미디어/사진",
        "psd" | "ai" | "sketch" | "fig" | "xd" => "미디어/디자인",
        // 영상/음악
        "mp4" | "mov" | "avi" | "mkv" | "webm" => "미디어/영상",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "미디어/음악",
        // 문서
        "pdf" | "doc" | "docx" | "hwp" | "hwpx" => "업무/문서",
        "xls" | "xlsx" | "csv" => "업무/문서",
        "ppt" | "pptx" | "key" => "업무/문서",
        // 코드/아카이브
        "zip" | "tar" | "gz" | "7z" | "rar" => "임시/압축",
        "dmg" | "pkg" | "app" => "임시/설치파일",
        // 설정
        "conf" | "toml" | "yaml" | "yml" => "시스템",
        // 폰트
        "ttf" | "otf" | "woff" | "woff2" => "미디어/디자인/폰트",
        // 기타
        _ => "임시",
    }
}

// 파일명 포맷: YYMMDD_설명.확장자
fn format_filename(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // 이미 포맷 준수하면 그대로
    let re_formatted = regex_lite::Regex::new(r"^\d{6}_").unwrap();
    if re_formatted.is_match(stem) {
        return path.file_name().unwrap().to_string_lossy().to_string();
    }

    // 날짜 접두사 추가
    let date = chrono_today();
    let clean_stem = stem
        .replace(' ', "_")
        .replace("__", "_");

    if ext.is_empty() {
        format!("{date}_{clean_stem}")
    } else {
        format!("{date}_{clean_stem}.{ext}")
    }
}

fn chrono_today() -> String {
    let output = Command::new("date")
        .args(["+%y%m%d"])
        .output()
        .expect("date 실행 실패");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn status() {
    println!("=== 파일 관리 상태 ===\n");

    let h = home();

    // Downloads 미정리 파일
    let dl_count = count_files(&format!("{h}/Downloads"));
    println!("[Downloads] {}개 파일", dl_count);

    // 임시 폴더
    let tmp_count = count_files(&format!("{h}/임시"));
    println!("[임시] {}개 파일", tmp_count);

    // LaunchAgent (자동 정리)
    let plist = format!("{h}/Library/LaunchAgents/com.mac-host.file-organizer.plist");
    println!("[자동 정리] {}", if Path::new(&plist).exists() { "✓ 활성화됨" } else { "✗ 비활성화" });

    // 각 폴더 상태
    println!("\n[폴더 현황]");
    let folders = [
        ("시스템", "설정, 바이너리, VPN"),
        ("프로젝트", "활성 코드"),
        ("업무", "업무 문서"),
        ("미디어", "사진, 영상, 디자인"),
        ("학습", "강의, 리서치"),
        ("인프라", "서버 설정"),
        ("창작", "소설, 게임, 영상"),
        ("사업", "비즈니스"),
        ("아카이브", "완료/보관"),
        ("임시", "미분류"),
    ];

    for (name, desc) in folders {
        let path = format!("{h}/{name}");
        let count = count_files(&path);
        let size = dir_size(&path);
        println!("  {name:8} — {desc:16} ({count}개, {size})");
    }
}

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
            // 폴더는 임시로
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

    // 정리 스크립트 생성
    let script_path = format!("{h}/시스템/bin/file-organizer.sh");
    common::ensure_dir(Path::new(&format!("{h}/시스템/bin")));

    let bin_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mac-host-commands"));
    let script = format!(r#"#!/bin/bash
# mac-host-commands 파일 자동 정리
# Downloads → 분류, 임시 30일+ → 아카이브

{bin} files organize 2>/dev/null
{bin} files cleanup-temp 2>/dev/null
"#, bin = bin_path.display());

    fs::write(&script_path, script).expect("스크립트 생성 실패");
    let _ = Command::new("chmod").args(["+x", &script_path]).output();

    // LaunchAgent — 매일 09:00 실행
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mac-host.file-organizer</string>
    <key>ProgramArguments</key>
    <array>
        <string>{script}</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>9</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>{home}/시스템/로그/file-organizer.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/시스템/로그/file-organizer.log</string>
</dict>
</plist>"#, script = script_path, home = h);

    common::ensure_dir(Path::new(&format!("{h}/시스템/로그")));
    fs::write(&plist_path, plist).expect("LaunchAgent 생성 실패");

    let _ = Command::new("launchctl").args(["load", &plist_path]).status();

    println!("[files] 자동 정리 설정 완료");
    println!("  매일 09:00 실행");
    println!("  Downloads → 자동 분류");
    println!("  임시/ 30일+ → 아카이브");
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

pub fn rename_format(dir: &str) {
    let h = home();
    let target = if dir.starts_with('/') || dir.starts_with('~') {
        dir.replace('~', &h)
    } else {
        format!("{h}/{dir}")
    };

    println!("[files] 파일명 포맷 적용: {target}\n");
    println!("  포맷: YYMMDD_설명.확장자\n");

    let entries: Vec<_> = fs::read_dir(&target)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .collect();

    let mut renamed = 0;
    for entry in entries {
        let path = entry.path();
        let old_name = entry.file_name().to_string_lossy().to_string();
        let new_name = format_filename(&path);

        if old_name != new_name {
            let new_path = path.parent().unwrap().join(&new_name);
            if !new_path.exists() {
                fs::rename(&path, &new_path).ok();
                println!("  {old_name} → {new_name}");
                renamed += 1;
            }
        }
    }

    if renamed == 0 {
        println!("  변경할 파일 없음 (이미 포맷 준수)");
    } else {
        println!("\n  {renamed}개 파일 이름 변경");
    }
}

const SD_PLIST: &str = "com.mac-host.sd-backup.plist";
const SD_SCRIPT: &str = "시스템/bin/sd-backup.sh";

pub fn sd_status() {
    let h = home();
    let plist = format!("{h}/Library/LaunchAgents/{SD_PLIST}");
    let enabled = Path::new(&plist).exists();

    println!("=== SD 자동 백업 ===\n");
    println!("[자동 백업] {}", if enabled { "✓ 활성화" } else { "✗ 비활성화" });

    // 백업 이력
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

    // 로그
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

fn count_files(dir: &str) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .count()
        })
        .unwrap_or(0)
}

fn dir_size(dir: &str) -> String {
    let (_, out) = common::run_cmd_quiet("du", &["-sh", dir]);
    out.split('\t').next().unwrap_or("0B").trim().to_string()
}
