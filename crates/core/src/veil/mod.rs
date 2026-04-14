use std::path::Path;
use std::process::Command;

use crate::common;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

use crate::constants::{VAULTCENTER_LXC, VAULTCENTER_URL, LOCALVAULT_URL};

pub fn status() {
    println!("=== VeilKey 상태 ===\n");

    // veilkey-cli
    let (has_cli, _) = common::run_cmd_quiet("which", &["veilkey-cli"]);
    println!("[veilkey-cli] {}", if has_cli { "✓ 설치됨" } else { "✗ 미설치" });

    // veil wrapper
    let (has_veil, _) = common::run_cmd_quiet("which", &["veil"]);
    println!("[veil] {}", if has_veil { "✓ 설치됨" } else { "✗ 미설치" });

    // LocalVault
    let (_, lv_resp) = common::run_cmd_quiet("curl", &["-s", &format!("{LOCALVAULT_URL}/health")]);
    let lv_ok = lv_resp.contains("ok");
    println!("[LocalVault] {LOCALVAULT_URL} {}", if lv_ok { "✓ 실행 중" } else { "✗ 미실행" });

    // veilkey-localvault 바이너리
    let (has_lv_bin, _) = common::run_cmd_quiet("which", &["veilkey-localvault"]);
    println!("[LocalVault 바이너리] {}", if has_lv_bin { "✓ 설치됨" } else { "✗ 미설치" });

    // LaunchAgent
    let plist = format!("{}/Library/LaunchAgents/com.veilkey.localvault.plist", home());
    let has_plist = Path::new(&plist).exists();
    println!("[LaunchAgent] {}", if has_plist { "✓ 등록됨 (부팅 시 자동 시작)" } else { "✗ 미등록" });

    // VaultCenter (직접 접근)
    let (_, vc_resp) = common::run_cmd_quiet("curl", &["-s", "--connect-timeout", "3", &format!("{VAULTCENTER_URL}/health")]);
    let vc_ok = vc_resp.contains("ok");
    println!("[VaultCenter] {VAULTCENTER_URL} (LXC {VAULTCENTER_LXC}) {}",
        if vc_ok { "✓ 실행 중" } else { "✗ 미실행" });

    // LocalVault → VaultCenter 연동
    if lv_ok {
        let (_, lv_status) = common::run_cmd_quiet("curl", &["-s", &format!("{LOCALVAULT_URL}/api/status")]);
        if lv_status.contains("vault") {
            println!("[연동] ✓ LocalVault → VaultCenter 연결됨");
        } else {
            println!("[연동] ✗ LocalVault → VaultCenter 미연결");
        }
    }

    // .veilkey/env 설정
    let env_files = [
        format!("{}/.veilkey/.veilkey/env", home()),
    ];
    for ef in &env_files {
        if Path::new(ef).exists() {
            let content = std::fs::read_to_string(ef).unwrap_or_default();
            if content.contains(LOCALVAULT_URL) {
                println!("[env] ✓ {ef}");
            } else {
                println!("[env] ⚠ {ef} (URL 불일치)");
            }
        }
    }

    // .veilkey.sh
    let has_profile = Path::new(&format!("{}/.veilkey.sh", home())).exists();
    println!("[셸 프로필] {}", if has_profile { "✓ ~/.veilkey.sh" } else { "✗ 미설정" });
}

pub fn install_cli() {
    let (has_cli, _) = common::run_cmd_quiet("which", &["veilkey-cli"]);
    if has_cli {
        println!("[veil] veilkey-cli 이미 설치됨");
        return;
    }

    println!("[veil] veilkey-cli 설치 중...");

    let cfg = crate::config::Config::load();
    let local_bin = format!("{}/.local/bin", home());
    common::ensure_dir(Path::new(&local_bin));

    let (ok, path) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user,
        "which veilkey-cli 2>/dev/null || find /opt/veilkey -name veilkey-cli -type f 2>/dev/null | head -1");

    if ok && !path.trim().is_empty() {
        let remote_path = path.trim();
        println!("[veil] Proxmox에서 바이너리 복사: {remote_path}");
        let (ok, _, _) = common::run_cmd("scp", &[
            &format!("{}@{}:{}", cfg.proxmox.user, cfg.proxmox.host, remote_path),
            &format!("{local_bin}/veilkey-cli"),
        ]);
        if ok {
            let _ = Command::new("chmod").args(["+x", &format!("{local_bin}/veilkey-cli")]).output();
            println!("[veil] veilkey-cli 설치 완료");
        }
    } else {
        eprintln!("[veil] Proxmox에서 veilkey-cli를 찾을 수 없습니다.");
        std::process::exit(1);
    }
}

pub fn install_localvault() {
    let (has_lv, _) = common::run_cmd_quiet("which", &["veilkey-localvault"]);
    if has_lv {
        println!("[veil] veilkey-localvault 이미 설치됨");
    } else {
        println!("[veil] veilkey-localvault 설치 중...");

        let cfg = crate::config::Config::load();
        let (ok, path) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user,
            "which veilkey-localvault 2>/dev/null || find /opt/veilkey -name veilkey-localvault -type f 2>/dev/null | head -1");

        if ok && !path.trim().is_empty() {
            let remote_path = path.trim();
            let (ok, _, _) = common::run_cmd("scp", &[
                &format!("{}@{}:{}", cfg.proxmox.user, cfg.proxmox.host, remote_path),
                "/usr/local/bin/veilkey-localvault",
            ]);
            if ok {
                let _ = Command::new("chmod").args(["+x", "/usr/local/bin/veilkey-localvault"]).output();
                println!("[veil] veilkey-localvault 설치 완료");
            }
        } else {
            eprintln!("[veil] Proxmox에서 veilkey-localvault를 찾을 수 없습니다.");
            std::process::exit(1);
        }
    }

    setup_launchagent();
}

fn setup_launchagent() {
    let plist_path = format!("{}/Library/LaunchAgents/com.veilkey.localvault.plist", home());
    if Path::new(&plist_path).exists() {
        println!("[veil] LaunchAgent 이미 등록됨");
        return;
    }

    println!("[veil] LaunchAgent 등록 중...");
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.veilkey.localvault</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/veilkey-localvault</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>VEILKEY_VAULTCENTER_URL</key>
        <string>{VAULTCENTER_URL}</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{home}/Library/Logs/veilkey-localvault.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/Library/Logs/veilkey-localvault.log</string>
</dict>
</plist>"#, home = home());

    std::fs::write(&plist_path, plist).expect("LaunchAgent plist 생성 실패");
    let _ = Command::new("launchctl").args(["load", &plist_path]).status();
    println!("[veil] LaunchAgent 등록 완료 (부팅 시 자동 시작)");
}

pub fn setup_env() {
    println!("[veil] .veilkey/env 파일 설정 중...");

    let env_content = format!(r#"#!/bin/sh
export VEILKEY_LOCALVAULT_URL="{LOCALVAULT_URL}"
export VEILKEY_VAULTCENTER_URL="{VAULTCENTER_URL}"
export VEILKEY_API="${{VEILKEY_LOCALVAULT_URL}}"
export VEILKEY_CLI_BIN={home}/.local/bin/veilkey-cli
"#, home = home());

    // 모든 .veilkey/env 파일 업데이트
    let dirs = [
        format!("{}/.veilkey/.veilkey", home()),
        format!("{}/veilkey/.veilkey", home()),
        format!("{}/veilkey-selfhosted/.veilkey", home()),
    ];

    for dir in &dirs {
        let env_path = format!("{dir}/env");
        if Path::new(dir).exists() {
            std::fs::write(&env_path, &env_content).unwrap_or_else(|e| {
                eprintln!("[veil] {env_path} 쓰기 실패: {e}");
            });
            println!("  ✓ {env_path}");
        }
    }
}

pub fn setup_profile() {
    let profile_path = format!("{}/.veilkey.sh", home());

    let content = format!(r#"# ── VeilKey Shell Profile ──────────────────────────────────────────
# mac-host-commands 에서 자동 생성

# 환경변수
export VEILKEY_LOCALVAULT_URL="{LOCALVAULT_URL}"
export VEILKEY_VAULTCENTER_URL="{VAULTCENTER_URL}"
export VEILKEY_API="${{VEILKEY_LOCALVAULT_URL}}"

# alias
alias vk='veilkey-cli'
alias vks='veilkey-cli scan'
alias vkf='veilkey-cli filter'
alias vkr='veilkey-cli resolve'
alias vkl='veilkey-cli list'
alias vkst='veilkey-cli status'
"#);

    std::fs::write(&profile_path, &content).expect(".veilkey.sh 생성 실패");
    println!("[veil] ~/.veilkey.sh 생성 완료");

    let zshrc = format!("{}/.zshrc", home());
    let zshrc_content = std::fs::read_to_string(&zshrc).unwrap_or_default();
    if !zshrc_content.contains(".veilkey.sh") {
        println!("[veil] .zshrc에 아래 라인을 추가하세요:");
        println!("  source ~/.veilkey.sh");
    } else {
        println!("[veil] .zshrc에 이미 source ~/.veilkey.sh 포함됨");
    }
}

pub fn check() {
    println!("[veil] 연결 파이프라인 점검 중...\n");

    // 1. VaultCenter 직접 접근
    let (_, vc_resp) = common::run_cmd_quiet("curl", &["-s", "--connect-timeout", "3", &format!("{VAULTCENTER_URL}/health")]);
    let vc_ok = vc_resp.contains("ok");
    println!("  1. VaultCenter ({VAULTCENTER_URL}) ... {}", if vc_ok { "OK" } else { "FAIL" });

    // 2. LocalVault 실행
    let (_, lv_resp) = common::run_cmd_quiet("curl", &["-s", &format!("{LOCALVAULT_URL}/health")]);
    let lv_ok = lv_resp.contains("ok");
    println!("  2. LocalVault ({LOCALVAULT_URL}) ... {}", if lv_ok { "OK" } else { "FAIL" });

    // 3. LocalVault → VaultCenter 연동
    if lv_ok {
        let (_, lv_status) = common::run_cmd_quiet("curl", &["-s", &format!("{LOCALVAULT_URL}/api/status")]);
        let linked = lv_status.contains("vault_node_uuid");
        println!("  3. LocalVault → VaultCenter 연동 ... {}", if linked { "OK" } else { "FAIL" });
    } else {
        println!("  3. LocalVault → VaultCenter 연동 ... SKIP (LocalVault 미실행)");
    }

    // 4. veilkey-cli 연결
    let (_, veil_out) = common::run_cmd_quiet("curl", &["-s", &format!("{LOCALVAULT_URL}/api/refs")]);
    let refs_ok = !veil_out.contains("404");
    println!("  4. API /api/refs ... {}", if refs_ok { "OK" } else { "FAIL (LocalVault 초기 설정 필요)" });

    if !vc_ok {
        println!("\n  [!] VaultCenter에 도달할 수 없습니다. WireGuard 연결을 확인하세요.");
    }
    if !lv_ok {
        println!("\n  [!] LocalVault가 실행되지 않았습니다: mac-host-commands veil start");
    }
    if lv_ok && !refs_ok {
        println!("\n  [!] LocalVault 초기 설정이 필요합니다:");
        println!("      브라우저에서 {LOCALVAULT_URL} 접속 후 VaultCenter URL 입력");
    }
}

pub fn localvault_start() {
    let (_, resp) = common::run_cmd_quiet("curl", &["-s", &format!("{LOCALVAULT_URL}/health")]);
    if resp.contains("ok") {
        println!("[veil] LocalVault 이미 실행 중");
        return;
    }

    println!("[veil] LocalVault 시작 중...");
    let plist = format!("{}/Library/LaunchAgents/com.veilkey.localvault.plist", home());
    if Path::new(&plist).exists() {
        let _ = Command::new("launchctl").args(["load", &plist]).status();
    } else {
        let _ = Command::new("veilkey-localvault")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    std::thread::sleep(std::time::Duration::from_secs(1));
    let (_, resp) = common::run_cmd_quiet("curl", &["-s", &format!("{LOCALVAULT_URL}/health")]);
    if resp.contains("ok") {
        println!("[veil] LocalVault 시작 완료");
    } else {
        eprintln!("[veil] LocalVault 시작 실패");
    }
}

pub fn localvault_stop() {
    let plist = format!("{}/Library/LaunchAgents/com.veilkey.localvault.plist", home());
    if Path::new(&plist).exists() {
        let _ = Command::new("launchctl").args(["unload", &plist]).status();
    } else {
        let _ = Command::new("pkill").args(["-f", "veilkey-localvault"]).status();
    }
    println!("[veil] LocalVault 중지 완료");
}

pub fn bootstrap() {
    println!("=== VeilKey 부트스트랩 ===\n");

    println!("--- [1/5] veilkey-cli 설치 ---");
    install_cli();

    println!("\n--- [2/5] LocalVault 설치 ---");
    install_localvault();

    println!("\n--- [3/5] env 파일 설정 ---");
    setup_env();

    println!("\n--- [4/5] 셸 프로필 설정 ---");
    setup_profile();

    println!("\n--- [5/5] 연결 점검 ---");
    check();

    println!("\n=== VeilKey 부트스트랩 완료 ===");
}
