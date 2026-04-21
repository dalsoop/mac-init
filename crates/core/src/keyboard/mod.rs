use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::common;
use crate::models::keyboard::KeyboardStatus;

const PLIST_LABEL: &str = "com.mai.keyboard-remap";
const LEGACY_PLIST_LABEL: &str = "com.mac-host-commands.keyboard-remap";

fn uid() -> String {
    Command::new("id")
        .args(["-u"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn launch_agents_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library/LaunchAgents")
}

fn plist_path_for(label: &str) -> PathBuf {
    launch_agents_dir().join(format!("{}.plist", label))
}

fn plist_path() -> PathBuf {
    plist_path_for(PLIST_LABEL)
}

fn legacy_plist_path() -> PathBuf {
    plist_path_for(LEGACY_PLIST_LABEL)
}

fn cleanup_launch_agent(_label: &str, path: &PathBuf) {
    if !path.exists() {
        return;
    }

    let _ = Command::new("launchctl")
        .args(["bootout", &format!("gui/{}", uid()), &path.to_string_lossy()])
        .output();
    let _ = fs::remove_file(path);
}

/// Query keyboard mapping status (no side effects)
pub fn get_status() -> KeyboardStatus {
    let (ok, stdout, _) = common::run_cmd("hidutil", &["property", "--get", "UserKeyMapping"]);
    let mapping_active = ok && stdout.contains("30064771181");
    let launch_agent_exists = plist_path().exists() || legacy_plist_path().exists();
    let karabiner_installed = std::path::Path::new("/Applications/Karabiner-Elements.app").exists();

    KeyboardStatus {
        mapping_active,
        launch_agent_exists,
        karabiner_installed,
    }
}

/// Apply hidutil mapping + create LaunchAgent
pub fn setup() -> Result<Vec<String>, String> {
    let mut log = Vec::new();

    // 1. Apply hidutil
    let (ok, _, stderr) = common::run_cmd(
        "hidutil",
        &[
            "property",
            "--set",
            r#"{"UserKeyMapping":[{"HIDKeyboardModifierMappingSrc":0x700000039,"HIDKeyboardModifierMappingDst":0x70000006D}]}"#,
        ],
    );
    if ok {
        log.push("Caps Lock → F18 적용됨".to_string());
    } else {
        return Err(format!("hidutil 적용 실패: {}", stderr.trim()));
    }

    // 2. Create LaunchAgent
    let plist = plist_path();
    cleanup_launch_agent(LEGACY_PLIST_LABEL, &legacy_plist_path());
    cleanup_launch_agent(PLIST_LABEL, &plist);
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

    fs::write(&plist, content).map_err(|e| format!("plist 생성 실패: {}", e))?;
    log.push(format!("LaunchAgent 생성: {}", plist.display()));

    // 3. Bootstrap + kickstart
    let bootstrap = Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{}", uid()), &plist.to_string_lossy()])
        .output()
        .map_err(|e| format!("LaunchAgent bootstrap 실패: {}", e))?;
    if bootstrap.status.success() {
        log.push("LaunchAgent 등록됨".to_string());
    } else {
        let stderr = String::from_utf8_lossy(&bootstrap.stderr).trim().to_string();
        log.push(format!("LaunchAgent bootstrap 경고: {}", stderr));
    }

    let kickstart = Command::new("launchctl")
        .args(["kickstart", "-k", &format!("gui/{}/{}", uid(), PLIST_LABEL)])
        .output()
        .map_err(|e| format!("LaunchAgent kickstart 실패: {}", e))?;
    if kickstart.status.success() {
        log.push("LaunchAgent 즉시 실행됨".to_string());
    } else {
        let stderr = String::from_utf8_lossy(&kickstart.stderr).trim().to_string();
        log.push(format!("LaunchAgent kickstart 경고: {}", stderr));
    }

    Ok(log)
}

/// Remove hidutil mapping + LaunchAgent
pub fn remove() -> Result<Vec<String>, String> {
    let mut log = Vec::new();

    let (ok, _, _) = common::run_cmd(
        "hidutil",
        &["property", "--set", r#"{"UserKeyMapping":[]}"#],
    );
    if ok {
        log.push("hidutil 매핑 해제됨".to_string());
    }

    let plist = plist_path();
    let legacy = legacy_plist_path();
    let had_current = plist.exists();
    let had_legacy = legacy.exists();

    cleanup_launch_agent(PLIST_LABEL, &plist);
    cleanup_launch_agent(LEGACY_PLIST_LABEL, &legacy);

    if had_current || had_legacy {
        log.push("LaunchAgent 제거됨".to_string());
    } else {
        log.push("LaunchAgent 없음 (이미 제거됨)".to_string());
    }

    Ok(log)
}

// === CLI 표시용 (하위 호환) ===
pub fn print_status() {
    let s = get_status();
    println!("=== 키보드 설정 ===\n");
    println!("[Caps Lock → F18] {}", if s.mapping_active { "✓ 적용됨 (hidutil)" } else { "✗ 미적용" });
    println!("[부팅 시 자동 적용] {}", if s.launch_agent_exists { "✓ LaunchAgent 등록됨" } else { "✗ LaunchAgent 없음" });
    if s.karabiner_installed {
        println!("[Karabiner] ⚠ 아직 설치되어 있음 (hidutil 사용 시 불필요)");
    } else {
        println!("[Karabiner] ✓ 미설치 (hidutil로 대체됨)");
    }
    println!("\n[입력 소스 단축키 확인]");
    println!("  시스템 설정 → 키보드 → 키보드 단축키 → 입력 소스");
    println!("  '이전 입력 소스 선택' = F18 인지 확인 필요");
}

pub fn print_setup() {
    println!("=== 키보드 설정: Caps Lock → F18 (한영 전환) ===\n");
    match setup() {
        Ok(logs) => {
            for l in &logs { println!("  ✓ {}", l); }
            println!("\n=== 완료 ===");
        }
        Err(e) => println!("  ✗ {}", e),
    }
}

pub fn print_remove() {
    println!("=== 키보드 매핑 제거 ===\n");
    match remove() {
        Ok(logs) => {
            for l in &logs { println!("  ✓ {}", l); }
            println!("\n=== 완료 ===");
        }
        Err(e) => println!("  ✗ {}", e),
    }
}
