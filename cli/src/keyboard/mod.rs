use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::common;

const PLIST_LABEL: &str = "com.mac-host-commands.keyboard-remap";

fn plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", PLIST_LABEL))
}

pub fn status() {
    println!("=== 키보드 설정 ===\n");

    // Check current hidutil mapping
    let (ok, stdout, _) = common::run_cmd("hidutil", &["property", "--get", "UserKeyMapping"]);
    let has_mapping = ok && stdout.contains("30064771181"); // F18 dst

    println!(
        "[Caps Lock → F18] {}",
        if has_mapping {
            "✓ 적용됨 (hidutil)"
        } else {
            "✗ 미적용"
        }
    );

    // Check LaunchAgent
    let plist = plist_path();
    let has_plist = plist.exists();
    println!(
        "[부팅 시 자동 적용] {}",
        if has_plist {
            "✓ LaunchAgent 등록됨"
        } else {
            "✗ LaunchAgent 없음"
        }
    );

    // Check if Karabiner is still installed
    let has_karabiner = std::path::Path::new("/Applications/Karabiner-Elements.app").exists();
    if has_karabiner {
        println!("[Karabiner] ⚠ 아직 설치되어 있음 (hidutil 사용 시 불필요)");
    } else {
        println!("[Karabiner] ✓ 미설치 (hidutil로 대체됨)");
    }

    // Check input source shortcut
    println!("\n[입력 소스 단축키 확인]");
    println!("  시스템 설정 → 키보드 → 키보드 단축키 → 입력 소스");
    println!("  '이전 입력 소스 선택' = F18 인지 확인 필요");
}

pub fn setup() {
    println!("=== 키보드 설정: Caps Lock → F18 (한영 전환) ===\n");

    // 1. Apply hidutil mapping
    println!("[1/3] Caps Lock → F18 매핑 적용 중...");
    let (ok, _, stderr) = common::run_cmd(
        "hidutil",
        &[
            "property",
            "--set",
            r#"{"UserKeyMapping":[{"HIDKeyboardModifierMappingSrc":0x700000039,"HIDKeyboardModifierMappingDst":0x70000006D}]}"#,
        ],
    );
    if ok {
        println!("  ✓ Caps Lock → F18 적용됨");
    } else {
        println!("  ✗ 적용 실패: {}", stderr.trim());
        return;
    }

    // 2. Create LaunchAgent
    println!("[2/3] LaunchAgent 생성 중...");
    let plist = plist_path();
    let content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/hidutil</string>
        <string>property</string>
        <string>--set</string>
        <string>{{"UserKeyMapping":[{{"HIDKeyboardModifierMappingSrc":0x700000039,"HIDKeyboardModifierMappingDst":0x70000006D}}]}}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#,
        PLIST_LABEL
    );

    if let Err(e) = fs::write(&plist, content) {
        println!("  ✗ plist 생성 실패: {}", e);
        return;
    }
    println!("  ✓ {}", plist.display());

    // 3. Load agent
    println!("[3/3] LaunchAgent 로드 중...");
    let _ = Command::new("launchctl")
        .args(["unload", &plist.to_string_lossy()])
        .output();
    let (ok, _, stderr) = common::run_cmd("launchctl", &["load", &plist.to_string_lossy()]);
    if ok {
        println!("  ✓ LaunchAgent 로드됨");
    } else {
        println!("  ✗ 로드 실패: {}", stderr.trim());
    }

    println!("\n=== 완료 ===");
    println!("  Caps Lock → F18 → 한영 전환");
    println!("  부팅 시 자동 적용됨");
    println!("\n  확인: 시스템 설정 → 키보드 → 키보드 단축키 → 입력 소스");
    println!("        '이전 입력 소스 선택' = F18 인지 확인");
}

pub fn remove() {
    println!("=== 키보드 매핑 제거 ===\n");

    // 1. Clear hidutil mapping
    println!("[1/2] hidutil 매핑 해제 중...");
    let (ok, _, _) = common::run_cmd(
        "hidutil",
        &["property", "--set", r#"{"UserKeyMapping":[]}"#],
    );
    if ok {
        println!("  ✓ 매핑 해제됨 (재부팅 전까지 유효)");
    }

    // 2. Remove LaunchAgent
    println!("[2/2] LaunchAgent 제거 중...");
    let plist = plist_path();
    if plist.exists() {
        let _ = Command::new("launchctl")
            .args(["unload", &plist.to_string_lossy()])
            .output();
        if let Err(e) = fs::remove_file(&plist) {
            println!("  ✗ 삭제 실패: {}", e);
        } else {
            println!("  ✓ LaunchAgent 제거됨");
        }
    } else {
        println!("  - LaunchAgent 없음 (이미 제거됨)");
    }

    println!("\n=== 완료 ===");
    println!("  Caps Lock은 기본 동작으로 복원됩니다.");
}
