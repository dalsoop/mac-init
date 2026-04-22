//! SVN CLI 래퍼 — 설치 여부, 버전, 서버 접속 체크

use mac_common::cmd;
use std::process::Command;

pub fn installed() -> bool {
    cmd::ok("svn", &["--version", "--quiet"])
}

pub fn version() -> String {
    cmd::stdout("svn", &["--version", "--quiet"])
}

pub fn install() {
    if installed() {
        println!("✓ svn 이미 설치됨 (v{})", version());
        return;
    }
    if !cmd::ok("brew", &["--version"]) {
        println!("✗ Homebrew 필요 → mai run bootstrap install");
        return;
    }
    println!("subversion 설치 중...");
    match Command::new("brew").args(["install", "subversion"]).status() {
        Ok(s) if s.success() => println!("✓ subversion 설치 완료 (v{})", version()),
        _ => println!("✗ 설치 실패"),
    }
}

pub fn check_server(url: &str) -> bool {
    let host = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or("");
    let check_url = format!("http://{}", host); // LINT_ALLOW: constructing URL for connectivity check
    Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "--connect-timeout", "3", &check_url])
        .output()
        .ok()
        .map(|o| {
            let code = String::from_utf8_lossy(&o.stdout).trim().to_string();
            matches!(code.as_str(), "200" | "401" | "301" | "302")
        })
        .unwrap_or(false)
}
