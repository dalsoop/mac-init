use std::path::Path;
use std::process::Command;

use crate::common;
use crate::constants::{
    CF_EMAIL, OPENCLAW_DOMAIN, OPENCLAW_GATEWAY_PORT, OPENCLAW_SUBDOMAIN, OPENCLAW_TUNNEL_NAME,
    OPENCLAW_ZONE_NAME, PLIST_CLOUDFLARED, PLIST_OPENCLAW_GATEWAY, PLIST_OPENCLAW_SYNC,
};

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

fn cf_api_key() -> String {
    std::env::var("CLOUD_FLARE_API_KEY").unwrap_or_default()
}

fn uid() -> String {
    Command::new("id")
        .args(["-u"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn cloudflared_dir() -> String {
    format!("{}/.cloudflared", home())
}

fn openclaw_dir() -> String {
    format!("{}/.openclaw", home())
}

// ─── Cloudflare API ──────────────────────────────────────

fn cf_api(method: &str, endpoint: &str, body: Option<&str>) -> (bool, String) {
    let url = format!("https://api.cloudflare.com/client/v4{endpoint}");
    let email_header = format!("X-Auth-Email: {CF_EMAIL}");
    let key_header = format!("X-Auth-Key: {}", cf_api_key());
    let mut args = vec![
        "-sf",
        "-X",
        method,
        &url,
        "-H",
        &email_header,
        "-H",
        &key_header,
        "-H",
        "Content-Type: application/json",
    ];
    let body_owned;
    if let Some(b) = body {
        args.push("-d");
        body_owned = b.to_string();
        args.push(&body_owned);
    }
    common::run_cmd_quiet("curl", &args)
}

fn json_extract(json: &str, key: &str) -> String {
    // python3으로 JSON 파싱
    let script = format!(
        "import sys,json; d=json.load(sys.stdin); r=d.get('result',d); \
         v=r[0]['{key}'] if isinstance(r,list) and r else r.get('{key}',''); \
         print(v)"
    );
    let mut child = Command::new("python3")
        .args(["-c", &script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("python3 실행 실패");

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(json.as_bytes());
    }
    let output = child.wait_with_output().expect("python3 실행 실패");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn json_extract_nested(json: &str, path: &str) -> String {
    let script = format!(
        "import sys,json; d=json.load(sys.stdin); r=d.get('result',d); \
         v=r[0] if isinstance(r,list) and r else r; \
         keys={path:?}.split('.'); \
         for k in keys: v=v[k] if isinstance(v,dict) and k in v else ''; \
         print(v)"
    );
    let mut child = Command::new("python3")
        .args(["-c", &script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("python3 실행 실패");

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(json.as_bytes());
    }
    let output = child.wait_with_output().expect("python3 실행 실패");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn json_result_empty(json: &str) -> bool {
    let script = "import sys,json; r=json.load(sys.stdin).get('result',[]); print('empty' if not r else 'has')";
    let mut child = Command::new("python3")
        .args(["-c", script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("python3 실행 실패");

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(json.as_bytes());
    }
    let output = child.wait_with_output().expect("python3 실행 실패");
    String::from_utf8_lossy(&output.stdout).trim() == "empty"
}

// ─── status ──────────────────────────────────────────────

pub fn status() {
    println!("=== OpenClaw 상태 ===\n");

    // openclaw CLI
    let (has_cli, ver) = common::run_cmd_quiet("openclaw", &["--version"]);
    println!(
        "[openclaw] {}",
        if has_cli {
            format!("✓ {}", ver.trim())
        } else {
            "✗ 미설치".to_string()
        }
    );

    // 게이트웨이
    let (_, resp) = common::run_cmd_quiet(
        "curl",
        &[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            &format!("http://127.0.0.1:{OPENCLAW_GATEWAY_PORT}/"),
        ],
    );
    let gw_ok = resp.trim() == "200";
    println!(
        "[게이트웨이] 127.0.0.1:{OPENCLAW_GATEWAY_PORT} {}",
        if gw_ok {
            "✓ 실행 중"
        } else {
            "✗ 미실행"
        }
    );

    // 게이트웨이 LaunchAgent
    let gw_plist = format!("{}/Library/LaunchAgents/{PLIST_OPENCLAW_GATEWAY}", home());
    println!(
        "[게이트웨이 서비스] {}",
        if Path::new(&gw_plist).exists() {
            "✓ LaunchAgent 등록됨"
        } else {
            "✗ 미등록"
        }
    );

    // cloudflared
    let (has_cf, _) = common::run_cmd_quiet("which", &["cloudflared"]);
    println!(
        "[cloudflared] {}",
        if has_cf {
            "✓ 설치됨"
        } else {
            "✗ 미설치"
        }
    );

    // cloudflared LaunchAgent
    let cf_plist = format!("{}/Library/LaunchAgents/{PLIST_CLOUDFLARED}", home());
    println!(
        "[cloudflared 서비스] {}",
        if Path::new(&cf_plist).exists() {
            "✓ LaunchAgent 등록됨"
        } else {
            "✗ 미등록"
        }
    );

    // 터널 상태 (외부 접속)
    let (_, ext_resp) = common::run_cmd_quiet(
        "curl",
        &[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "--connect-timeout",
            "5",
            &format!("https://{OPENCLAW_DOMAIN}/"),
        ],
    );
    let ext_ok = ext_resp.trim() == "200";
    println!(
        "[외부 접속] https://{OPENCLAW_DOMAIN}/ {}",
        if ext_ok {
            "✓ 정상"
        } else {
            "✗ 연결 불가"
        }
    );

    // config 파일
    let cfg_path = format!("{}/openclaw.json", openclaw_dir());
    println!(
        "[설정] {}",
        if Path::new(&cfg_path).exists() {
            "✓ ~/.openclaw/openclaw.json"
        } else {
            "✗ 미설정"
        }
    );
}

// ─── install ─────────────────────────────────────────────

pub fn install(telegram_token: Option<&str>) {
    println!("=== OpenClaw 설치 ===\n");

    println!("--- [1/7] OpenClaw CLI 설치 ---");
    install_cli();

    println!("\n--- [2/7] 초기 설정 + Claude CLI Backend ---");
    onboard();

    println!("\n--- [3/7] 외부 접근 설정 ---");
    configure_control_ui();

    println!("\n--- [4/7] 게이트웨이 서비스 등록 ---");
    install_gateway_service();

    println!("\n--- [5/7] Cloudflare Tunnel 설정 ---");
    setup_tunnel();

    println!("\n--- [6/7] 텔레그램 연동 ---");
    if let Some(token) = telegram_token {
        setup_telegram(token);
    } else {
        println!("[openclaw] --telegram-token 미지정 — 건너뜀");
        println!("[openclaw] 나중에: mac-host-commands openclaw telegram --token <봇토큰>");
    }

    println!("\n--- [7/7] 검증 ---");
    verify();

    println!("\n=== OpenClaw 설치 완료 ===");
    println!("  대시보드: https://{OPENCLAW_DOMAIN}/");
    println!("  로컬:     http://127.0.0.1:{OPENCLAW_GATEWAY_PORT}/");
    println!("  모델:     claude-cli/claude-sonnet-4-6 (구독제)");
}

fn install_cli() {
    let (has, _) = common::run_cmd_quiet("which", &["openclaw"]);
    if has {
        println!("[openclaw] 이미 설치됨, 업그레이드 중...");
    }

    let (ok, _, _) = common::run_cmd(
        "bash",
        &["-c", "curl -fsSL https://openclaw.ai/install.sh | bash"],
    );
    if ok {
        println!("[openclaw] CLI 설치 완료");
    } else {
        eprintln!("[openclaw] CLI 설치 실패");
        std::process::exit(1);
    }
}

fn onboard() {
    let (ok, _, _) = common::run_cmd(
        "openclaw",
        &[
            "onboard",
            "--mode",
            "local",
            "--non-interactive",
            "--accept-risk",
            "--skip-channels",
            "--skip-skills",
            "--skip-search",
            "--skip-ui",
            "--skip-health",
            "--auth-choice",
            "skip",
            "--skip-daemon",
        ],
    );
    if ok {
        println!("[openclaw] 초기 설정 완료 (로컬 모드)");
    } else {
        eprintln!("[openclaw] 초기 설정 실패");
    }

    // Claude 인증 설정 (Keychain에서 OAuth 토큰 추출)
    setup_claude_auth();
}

fn setup_claude_auth() {
    let home = home();
    // Keychain에서 OAuth 토큰 추출
    if sync_claude_from_keychain(&home) {
        let (_, _, _) = common::run_cmd(
            "openclaw",
            &[
                "config",
                "set",
                "agents.defaults.model",
                r#"{"primary":"anthropic/claude-sonnet-4-6"}"#,
            ],
        );
        println!("[openclaw] ✓ 모델: anthropic/claude-sonnet-4-6 (구독제 OAuth)");
    } else {
        println!(
            "[openclaw] Claude Code 로그인 후 `mac-host-commands openclaw sync-auth` 실행하세요"
        );
    }
}

fn configure_control_ui() {
    let cmds = [
        (
            "gateway.controlUi.allowedOrigins",
            format!("[\"https://{OPENCLAW_DOMAIN}\"]"),
        ),
        ("gateway.controlUi.allowInsecureAuth", "true".to_string()),
        (
            "gateway.controlUi.dangerouslyDisableDeviceAuth",
            "true".to_string(),
        ),
    ];

    for (key, val) in &cmds {
        let mut args = vec!["config", "set", key, val.as_str()];
        if *key != "gateway.controlUi.allowedOrigins" {
            args.push("--strict-json");
        }
        let (ok, _, _) = common::run_cmd("openclaw", &args);
        if !ok {
            eprintln!("[openclaw] {key} 설정 실패");
        }
    }
    println!("[openclaw] Control UI 외부 접근 설정 완료");
}

fn install_gateway_service() {
    let (ok, _, _) = common::run_cmd("openclaw", &["daemon", "install"]);
    if ok {
        println!("[openclaw] 게이트웨이 LaunchAgent 등록 완료");
    } else {
        eprintln!("[openclaw] 게이트웨이 LaunchAgent 등록 실패");
    }
}

fn setup_tunnel() {
    let api_key = cf_api_key();
    if api_key.is_empty() {
        eprintln!("[openclaw] CLOUD_FLARE_API_KEY가 설정되지 않았습니다 (~/.env)");
        std::process::exit(1);
    }

    // cloudflared 설치
    let (has_cf, _) = common::run_cmd_quiet("which", &["cloudflared"]);
    if !has_cf {
        println!("[openclaw] cloudflared 설치 중...");
        let (ok, _, _) = common::run_cmd("brew", &["install", "cloudflared"]);
        if !ok {
            eprintln!("[openclaw] cloudflared 설치 실패");
            std::process::exit(1);
        }
    }
    println!("[openclaw] cloudflared 설치 확인");

    // Zone/Account ID 조회
    let (ok, zone_resp) = cf_api("GET", &format!("/zones?name={OPENCLAW_ZONE_NAME}"), None);
    if !ok {
        eprintln!("[openclaw] Cloudflare Zone 조회 실패");
        std::process::exit(1);
    }
    let zone_id = json_extract(&zone_resp, "id");
    let account_id = json_extract_nested(&zone_resp, "account.id");
    println!("[openclaw] Zone: {OPENCLAW_ZONE_NAME} ({zone_id})");

    // 기존 터널 확인 또는 생성
    let (_, tunnel_resp) = cf_api(
        "GET",
        &format!("/accounts/{account_id}/cfd_tunnel?name={OPENCLAW_TUNNEL_NAME}&is_deleted=false"),
        None,
    );
    let tunnel_id;
    let tunnel_secret;

    if json_result_empty(&tunnel_resp) {
        // 새 터널 생성
        let (secret_ok, secret) = common::run_cmd_quiet("openssl", &["rand", "-base64", "32"]);
        if !secret_ok {
            eprintln!("[openclaw] openssl 시크릿 생성 실패");
            std::process::exit(1);
        }
        tunnel_secret = secret.trim().to_string();

        let body =
            format!(r#"{{"name":"{OPENCLAW_TUNNEL_NAME}","tunnel_secret":"{tunnel_secret}"}}"#);
        let (ok, create_resp) = cf_api(
            "POST",
            &format!("/accounts/{account_id}/cfd_tunnel"),
            Some(&body),
        );
        if !ok {
            eprintln!("[openclaw] 터널 생성 실패");
            std::process::exit(1);
        }
        tunnel_id = json_extract(&create_resp, "id");
        println!("[openclaw] 터널 '{OPENCLAW_TUNNEL_NAME}' 생성 ({tunnel_id})");
    } else {
        tunnel_id = json_extract(&tunnel_resp, "id");
        tunnel_secret = json_extract_nested(&tunnel_resp, "credentials_file.TunnelSecret");
        println!("[openclaw] 터널 '{OPENCLAW_TUNNEL_NAME}' 이미 존재 ({tunnel_id})");
    }

    // credentials 파일
    let cf_dir = cloudflared_dir();
    common::ensure_dir(Path::new(&cf_dir));

    let cred_path = format!("{cf_dir}/{tunnel_id}.json");
    let cred = format!(
        r#"{{"AccountTag":"{}","TunnelID":"{}","TunnelName":"{}","TunnelSecret":"{}"}}"#,
        account_id, tunnel_id, OPENCLAW_TUNNEL_NAME, tunnel_secret
    );
    std::fs::write(&cred_path, &cred).expect("credentials 파일 생성 실패");

    // config.yml
    let config = format!(
        "tunnel: {tunnel_id}\n\
         credentials-file: {cred_path}\n\n\
         ingress:\n  \
         - hostname: {OPENCLAW_DOMAIN}\n    \
         service: http://127.0.0.1:{OPENCLAW_GATEWAY_PORT}\n  \
         - service: http_status:404\n"
    );
    std::fs::write(&format!("{cf_dir}/config.yml"), &config).expect("config.yml 생성 실패");
    println!("[openclaw] cloudflared 설정 파일 생성 완료");

    // DNS CNAME 레코드
    let (_, dns_resp) = cf_api(
        "GET",
        &format!("/zones/{zone_id}/dns_records?type=CNAME&name={OPENCLAW_DOMAIN}"),
        None,
    );

    if json_result_empty(&dns_resp) {
        let body = format!(
            r#"{{"type":"CNAME","name":"{}","content":"{}.cfargotunnel.com","proxied":true}}"#,
            OPENCLAW_SUBDOMAIN, tunnel_id
        );
        let (ok, _) = cf_api(
            "POST",
            &format!("/zones/{zone_id}/dns_records"),
            Some(&body),
        );
        if ok {
            println!("[openclaw] DNS 레코드 생성: {OPENCLAW_DOMAIN}");
        } else {
            eprintln!("[openclaw] DNS 레코드 생성 실패");
        }
    } else {
        let dns_id = json_extract(&dns_resp, "id");
        let body = format!(
            r#"{{"type":"CNAME","name":"{}","content":"{}.cfargotunnel.com","proxied":true}}"#,
            OPENCLAW_SUBDOMAIN, tunnel_id
        );
        let _ = cf_api(
            "PUT",
            &format!("/zones/{zone_id}/dns_records/{dns_id}"),
            Some(&body),
        );
        println!("[openclaw] DNS 레코드 업데이트: {OPENCLAW_DOMAIN}");
    }

    // cloudflared LaunchAgent
    setup_cloudflared_service();
}

fn setup_cloudflared_service() {
    let plist_path = format!("{}/Library/LaunchAgents/{PLIST_CLOUDFLARED}", home());

    // 기존 서비스 중지
    let _ = Command::new("launchctl")
        .args([
            "bootout",
            &format!("gui/{}", uid()),
            "com.cloudflare.cloudflared",
        ])
        .output();

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cloudflare.cloudflared</string>
    <key>ProgramArguments</key>
    <array>
        <string>/opt/homebrew/bin/cloudflared</string>
        <string>tunnel</string>
        <string>run</string>
        <string>{OPENCLAW_TUNNEL_NAME}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{home}/Library/Logs/com.cloudflare.cloudflared.out.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/Library/Logs/com.cloudflare.cloudflared.err.log</string>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>ThrottleInterval</key>
    <integer>5</integer>
</dict>
</plist>"#,
        home = home()
    );

    std::fs::write(&plist_path, &plist).expect("cloudflared plist 생성 실패");

    let _ = Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{}", uid()), &plist_path])
        .status();
    println!("[openclaw] cloudflared LaunchAgent 등록 완료");
}

fn verify() {
    std::thread::sleep(std::time::Duration::from_secs(3));

    // 게이트웨이
    let (_, resp) = common::run_cmd_quiet(
        "curl",
        &[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            &format!("http://127.0.0.1:{OPENCLAW_GATEWAY_PORT}/"),
        ],
    );
    let gw_ok = resp.trim() == "200";
    println!(
        "[검증] 게이트웨이 ... {}",
        if gw_ok { "OK" } else { "FAIL" }
    );

    // 외부 접속
    let (_, ext_resp) = common::run_cmd_quiet(
        "curl",
        &[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "--connect-timeout",
            "5",
            &format!("https://{OPENCLAW_DOMAIN}/"),
        ],
    );
    let ext_ok = ext_resp.trim() == "200";
    println!(
        "[검증] https://{OPENCLAW_DOMAIN}/ ... {}",
        if ext_ok {
            "OK"
        } else {
            "FAIL (터널 연결 대기 중일 수 있음)"
        }
    );
}

// ─── uninstall ───────────────────────────────────────────

pub fn uninstall() {
    println!("=== OpenClaw 삭제 ===\n");

    println!("--- [1/4] 서비스 중지 ---");
    stop_services();

    println!("\n--- [2/4] Cloudflare 리소스 삭제 ---");
    cleanup_cloudflare();

    println!("\n--- [3/4] 로컬 데이터 삭제 ---");
    cleanup_local();

    println!("\n--- [4/4] 패키지 제거 ---");
    uninstall_packages();

    println!("\n=== OpenClaw 삭제 완료 ===");
}

fn stop_services() {
    // 게이트웨이 프로세스 종료
    let _ = Command::new("bash")
        .args([
            "-c",
            &format!("kill $(lsof -ti :{OPENCLAW_GATEWAY_PORT}) 2>/dev/null"),
        ])
        .output();
    println!("[openclaw] 게이트웨이 프로세스 종료");

    // LaunchAgent 제거
    let uid = Command::new("id")
        .args(["-u"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let _ = Command::new("launchctl")
        .args(["bootout", &format!("gui/{uid}"), "ai.openclaw.gateway"])
        .output();
    let gw_plist = format!("{}/Library/LaunchAgents/{PLIST_OPENCLAW_GATEWAY}", home());
    let _ = std::fs::remove_file(&gw_plist);
    println!("[openclaw] 게이트웨이 LaunchAgent 제거");

    let _ = Command::new("launchctl")
        .args([
            "bootout",
            &format!("gui/{uid}"),
            "com.cloudflare.cloudflared",
        ])
        .output();
    let cf_plist = format!("{}/Library/LaunchAgents/{PLIST_CLOUDFLARED}", home());
    let _ = std::fs::remove_file(&cf_plist);
    println!("[openclaw] cloudflared LaunchAgent 제거");
}

fn cleanup_cloudflare() {
    let api_key = cf_api_key();
    if api_key.is_empty() {
        println!("[openclaw] CLOUD_FLARE_API_KEY 없음 — Cloudflare 원격 정리 건너뜀");
        return;
    }

    let (ok, zone_resp) = cf_api("GET", &format!("/zones?name={OPENCLAW_ZONE_NAME}"), None);
    if !ok {
        println!("[openclaw] Cloudflare API 접근 실패 — 원격 정리 건너뜀");
        return;
    }

    let zone_id = json_extract(&zone_resp, "id");
    let account_id = json_extract_nested(&zone_resp, "account.id");

    // DNS 레코드 삭제
    if !zone_id.is_empty() {
        let (_, dns_resp) = cf_api(
            "GET",
            &format!("/zones/{zone_id}/dns_records?type=CNAME&name={OPENCLAW_DOMAIN}"),
            None,
        );
        if !json_result_empty(&dns_resp) {
            let dns_id = json_extract(&dns_resp, "id");
            let (ok, _) = cf_api(
                "DELETE",
                &format!("/zones/{zone_id}/dns_records/{dns_id}"),
                None,
            );
            println!(
                "[openclaw] DNS 레코드 {} {OPENCLAW_DOMAIN}",
                if ok { "삭제 완료" } else { "삭제 실패" }
            );
        }
    }

    // 터널 삭제
    if !account_id.is_empty() {
        let (_, tunnel_resp) = cf_api(
            "GET",
            &format!(
                "/accounts/{account_id}/cfd_tunnel?name={OPENCLAW_TUNNEL_NAME}&is_deleted=false"
            ),
            None,
        );
        if !json_result_empty(&tunnel_resp) {
            let tunnel_id = json_extract(&tunnel_resp, "id");
            let (ok, _) = cf_api(
                "DELETE",
                &format!("/accounts/{account_id}/cfd_tunnel/{tunnel_id}?cascade=true"),
                None,
            );
            println!(
                "[openclaw] 터널 '{OPENCLAW_TUNNEL_NAME}' {}",
                if ok {
                    "삭제 완료"
                } else {
                    "삭제 실패 (활성 연결이 있을 수 있음)"
                }
            );
        }
    }
}

fn cleanup_local() {
    // ~/.openclaw
    let oc_dir = openclaw_dir();
    if Path::new(&oc_dir).exists() {
        let _ = std::fs::remove_dir_all(&oc_dir);
        println!("[openclaw] ~/.openclaw 삭제");
    }

    // ~/.cloudflared
    let cf_dir = cloudflared_dir();
    if Path::new(&cf_dir).exists() {
        let _ = std::fs::remove_dir_all(&cf_dir);
        println!("[openclaw] ~/.cloudflared 삭제");
    }

    // 로그
    let logs = [
        "Library/Logs/com.cloudflare.cloudflared.out.log",
        "Library/Logs/com.cloudflare.cloudflared.err.log",
    ];
    for log in &logs {
        let p = format!("{}/{log}", home());
        let _ = std::fs::remove_file(&p);
    }
    println!("[openclaw] 로그 파일 삭제");
}

fn uninstall_packages() {
    // openclaw npm 제거
    let (ok, _, _) = common::run_cmd("npm", &["uninstall", "-g", "openclaw"]);
    println!(
        "[openclaw] npm 패키지 {}",
        if ok {
            "제거 완료"
        } else {
            "제거 실패 (이미 없거나)"
        }
    );

    // cloudflared brew 제거
    let (ok, _, _) = common::run_cmd("brew", &["uninstall", "cloudflared"]);
    println!(
        "[openclaw] cloudflared {}",
        if ok {
            "제거 완료"
        } else {
            "제거 실패 (이미 없거나)"
        }
    );
}

// ─── start / stop ────────────────────────────────────────

pub fn start() {
    // 게이트웨이
    let (_, resp) = common::run_cmd_quiet(
        "curl",
        &[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            &format!("http://127.0.0.1:{OPENCLAW_GATEWAY_PORT}/"),
        ],
    );
    if resp.trim() == "200" {
        println!("[openclaw] 게이트웨이 이미 실행 중");
    } else {
        let gw_plist = format!("{}/Library/LaunchAgents/{PLIST_OPENCLAW_GATEWAY}", home());
        if Path::new(&gw_plist).exists() {
            let _ = Command::new("launchctl").args(["load", &gw_plist]).status();
        } else {
            let _ = Command::new("openclaw")
                .args(["gateway", "--port", &OPENCLAW_GATEWAY_PORT.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!("[openclaw] 게이트웨이 시작");
    }

    // cloudflared
    let cf_plist = format!("{}/Library/LaunchAgents/{PLIST_CLOUDFLARED}", home());
    if Path::new(&cf_plist).exists() {
        let _ = Command::new("launchctl").args(["load", &cf_plist]).status();
        println!("[openclaw] cloudflared 시작");
    }
}

pub fn stop() {
    let _ = Command::new("bash")
        .args([
            "-c",
            &format!("kill $(lsof -ti :{OPENCLAW_GATEWAY_PORT}) 2>/dev/null"),
        ])
        .output();
    println!("[openclaw] 게이트웨이 중지");

    let cf_plist = format!("{}/Library/LaunchAgents/{PLIST_CLOUDFLARED}", home());
    if Path::new(&cf_plist).exists() {
        let _ = Command::new("launchctl")
            .args(["unload", &cf_plist])
            .status();
    } else {
        let _ = Command::new("pkill").args(["-f", "cloudflared"]).status();
    }
    println!("[openclaw] cloudflared 중지");
}

// ─── sync-auth ───────────────────────────────────────────

pub fn sync_auth() {
    println!("=== OpenClaw 인증 동기화 ===\n");

    let home = home();
    let mut synced = 0;

    // 1. Claude Code → OpenClaw (Keychain에서 OAuth 토큰 추출)
    println!("--- Claude Code (Keychain → auth-profiles.json) ---");
    if sync_claude_from_keychain(&home) {
        synced += 1;
    }

    // 2. Codex → OpenClaw (auth.json 복사 + auth-profiles.json 등록)
    println!("\n--- Codex ---");
    if sync_codex(&home) {
        synced += 1;
    }

    // 3. 모델 설정 (anthropic 직접 호출)
    if synced > 0 {
        let (_, _, _) = common::run_cmd(
            "openclaw",
            &[
                "config",
                "set",
                "agents.defaults.model",
                r#"{"primary":"anthropic/claude-sonnet-4-6"}"#,
            ],
        );
        println!("\n[sync] 모델: anthropic/claude-sonnet-4-6");
    }

    println!("\n=== 동기화 완료: {synced}개 연동됨 ===");

    if synced > 0 {
        restart_gateway();
    }
}

/// macOS Keychain에서 Claude Code OAuth 토큰을 추출하여 auth-profiles.json에 저장
fn sync_claude_from_keychain(home: &str) -> bool {
    // Keychain에서 credentials 추출
    let (ok, cred_json) = common::run_cmd_quiet(
        "security",
        &[
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ],
    );
    if !ok || cred_json.trim().is_empty() {
        println!("[sync] ✗ Keychain에 Claude Code 인증 없음 (로그인 필요)");
        return false;
    }

    // JSON 파싱 → auth-profiles.json에 쓰기
    let script = r#"
import sys, json, os

cred = json.loads(sys.argv[1])
oauth = cred.get("claudeAiOauth", {})
access = oauth.get("accessToken", "")
refresh = oauth.get("refreshToken", "")
expires = oauth.get("expiresAt", 0)
sub_type = oauth.get("subscriptionType", "unknown")

if not access:
    print("NO_TOKEN")
    sys.exit(1)

profiles_path = sys.argv[2]
if os.path.exists(profiles_path):
    with open(profiles_path) as f:
        profiles = json.load(f)
else:
    profiles = {"version": 1, "profiles": {}}

profiles["profiles"]["anthropic:default"] = {
    "type": "oauth",
    "provider": "anthropic",
    "access": access,
    "refresh": refresh,
    "expires": expires
}

with open(profiles_path, "w") as f:
    json.dump(profiles, f, indent=2)

print(f"OK:{sub_type}:{expires}")
"#;

    let profiles_path = format!("{home}/.openclaw/agents/main/agent/auth-profiles.json");
    common::ensure_dir(Path::new(&format!("{home}/.openclaw/agents/main/agent")));

    let (ok, result) =
        common::run_cmd_quiet("python3", &["-c", script, cred_json.trim(), &profiles_path]);

    if ok && result.starts_with("OK:") {
        let parts: Vec<&str> = result.trim().split(':').collect();
        let sub_type = parts.get(1).unwrap_or(&"");
        println!("[sync] ✓ Claude Code OAuth 토큰 → auth-profiles.json");
        println!("[sync]   구독: {sub_type}");
        let _ = Command::new("chmod").args(["600", &profiles_path]).output();
        true
    } else {
        eprintln!("[sync] ✗ Claude Code 토큰 추출 실패");
        false
    }
}

/// Codex auth.json을 OpenClaw auth-profiles.json에 등록
fn sync_codex(home: &str) -> bool {
    let (has_codex, _) = common::run_cmd_quiet("which", &["codex"]);
    let codex_auth = format!("{home}/.codex/auth.json");
    if !has_codex {
        println!("[sync] ✗ Codex CLI 미설치");
        return false;
    }
    if !Path::new(&codex_auth).exists() {
        println!("[sync] ✗ Codex auth.json 없음");
        return false;
    }

    // auth-profiles.json에 codex OAuth 등록
    let script = r#"
import sys, json, os

codex_path = sys.argv[1]
profiles_path = sys.argv[2]

with open(codex_path) as f:
    codex = json.load(f)

tokens = codex.get("tokens", {})
access = tokens.get("access_token", "")
refresh = tokens.get("refresh_token", "")

if not access:
    print("NO_TOKEN")
    sys.exit(1)

if os.path.exists(profiles_path):
    with open(profiles_path) as f:
        profiles = json.load(f)
else:
    profiles = {"version": 1, "profiles": {}}

profiles["profiles"]["openai-codex:default"] = {
    "type": "oauth",
    "provider": "openai-codex",
    "access": access,
    "refresh": refresh,
    "accountId": codex.get("tokens", {}).get("account_id", "")
}

with open(profiles_path, "w") as f:
    json.dump(profiles, f, indent=2)

print("OK")
"#;

    let profiles_path = format!("{home}/.openclaw/agents/main/agent/auth-profiles.json");
    let (ok, _) = common::run_cmd_quiet("python3", &["-c", script, &codex_auth, &profiles_path]);

    if ok {
        println!("[sync] ✓ Codex OAuth → auth-profiles.json");
        true
    } else {
        eprintln!("[sync] ✗ Codex 토큰 등록 실패");
        false
    }
}

/// sync-auth 자동 실행 LaunchAgent 등록 (30분마다)
pub fn sync_auth_auto() {
    let plist_path = format!("{}/Library/LaunchAgents/{}", home(), PLIST_OPENCLAW_SYNC);
    let mac_host_bin = which_mac_host_commands();

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mac-host.openclaw-sync</string>
    <key>ProgramArguments</key>
    <array>
        <string>{mac_host_bin}</string>
        <string>openclaw</string>
        <string>sync-auth</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:{home}/.local/bin:{home}/.cargo/bin</string>
    </dict>
    <key>StartInterval</key>
    <integer>1800</integer>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{home}/Library/Logs/openclaw-sync.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/Library/Logs/openclaw-sync.log</string>
</dict>
</plist>"#,
        mac_host_bin = mac_host_bin,
        home = home()
    );

    std::fs::write(&plist_path, &plist).expect("plist 생성 실패");

    let _ = Command::new("launchctl")
        .args([
            "bootout",
            &format!("gui/{}", uid()),
            "com.mac-host.openclaw-sync",
        ])
        .output();
    let _ = Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{}", uid()), &plist_path])
        .status();
    println!("[openclaw] ✓ sync-auth 자동 실행 등록 (30분마다)");
    println!("[openclaw]   로그: ~/Library/Logs/openclaw-sync.log");
}

pub fn sync_auth_disable() {
    let _ = Command::new("launchctl")
        .args([
            "bootout",
            &format!("gui/{}", uid()),
            "com.mac-host.openclaw-sync",
        ])
        .output();
    let plist_path = format!("{}/Library/LaunchAgents/{}", home(), PLIST_OPENCLAW_SYNC);
    let _ = std::fs::remove_file(&plist_path);
    println!("[openclaw] ✓ sync-auth 자동 실행 해제");
}

fn which_mac_host_commands() -> String {
    let (ok, path) = common::run_cmd_quiet("which", &["mac-host-commands"]);
    if ok {
        path.trim().to_string()
    } else {
        // cargo install 경로 fallback
        format!("{}/.cargo/bin/mac-host-commands", home())
    }
}

// ─── telegram ────────────────────────────────────────────

fn setup_telegram(token: &str) {
    let (ok, _, _) = common::run_cmd(
        "openclaw",
        &["channels", "add", "--channel", "telegram", "--token", token],
    );
    if ok {
        println!("[openclaw] ✓ 텔레그램 봇 등록 완료");
        println!("[openclaw]   봇에 DM 보내면 페어링 코드가 나옵니다");
        println!("[openclaw]   승인: mac-host-commands openclaw telegram-approve <코드>");
    } else {
        eprintln!("[openclaw] ✗ 텔레그램 봇 등록 실패");
    }
}

pub fn telegram(token: &str) {
    setup_telegram(token);
    restart_gateway();
}

pub fn telegram_approve(code: &str) {
    let (ok, _, _) = common::run_cmd("openclaw", &["pairing", "approve", "telegram", code]);
    if ok {
        println!("[openclaw] ✓ 텔레그램 페어링 승인 완료");
    } else {
        eprintln!("[openclaw] ✗ 페어링 승인 실패");
    }
}

// ─── helpers ─────────────────────────────────────────────

fn restart_gateway() {
    println!("[openclaw] 게이트웨이 재시작 중...");
    let _ = Command::new("bash")
        .args([
            "-c",
            &format!("kill $(lsof -ti :{OPENCLAW_GATEWAY_PORT}) 2>/dev/null"),
        ])
        .output();
    std::thread::sleep(std::time::Duration::from_secs(3));
    // LaunchAgent가 자동 재시작
    let (_, resp) = common::run_cmd_quiet(
        "curl",
        &[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            &format!("http://127.0.0.1:{OPENCLAW_GATEWAY_PORT}/"),
        ],
    );
    if resp.trim() == "200" {
        println!("[openclaw] ✓ 게이트웨이 재시작 완료");
    } else {
        println!("[openclaw] ! 게이트웨이 재시작 대기 중 (LaunchAgent가 자동 시작)");
    }
}

// ─── exec approvals ─────────────────────────────────────

fn exec_approvals_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/.openclaw/exec-approvals.json", home)
}

pub fn exec_approve() {
    let content = std::fs::read_to_string(&exec_approvals_path()).unwrap_or_default();
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or_else(|_| serde_json::json!({"version": 1, "defaults": {}, "agents": {}}));

    json["defaults"]["security"] = serde_json::json!("full");
    json["defaults"]["ask"] = serde_json::json!("off");

    let output = serde_json::to_string_pretty(&json).unwrap();
    std::fs::write(&exec_approvals_path(), &output).expect("exec-approvals.json 쓰기 실패");
    println!("[openclaw] exec 자동 승인 활성화 (security=full, ask=off)");
    println!("  모든 명령이 승인 없이 실행됩니다.");
}

pub fn exec_ask() {
    let content = std::fs::read_to_string(&exec_approvals_path()).unwrap_or_default();
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or_else(|_| serde_json::json!({"version": 1, "defaults": {}, "agents": {}}));

    json["defaults"]["security"] = serde_json::json!("allowlist");
    json["defaults"]["ask"] = serde_json::json!("on-miss");

    let output = serde_json::to_string_pretty(&json).unwrap();
    std::fs::write(&exec_approvals_path(), &output).expect("exec-approvals.json 쓰기 실패");
    println!("[openclaw] exec 확인 모드 활성화 (security=allowlist, ask=on-miss)");
    println!("  allowlist에 없는 명령은 실행 전 승인 요청합니다.");
}

pub fn exec_status() {
    let content = match std::fs::read_to_string(&exec_approvals_path()) {
        Ok(c) => c,
        Err(_) => {
            println!("[openclaw] exec-approvals.json 없음 (기본값 사용)");
            return;
        }
    };
    let json: serde_json::Value =
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));

    let security = json["defaults"]["security"].as_str().unwrap_or("미설정");
    let ask = json["defaults"]["ask"].as_str().unwrap_or("미설정");
    println!("[openclaw] exec 설정:");
    println!("  security = {security}");
    println!("  ask      = {ask}");
}
