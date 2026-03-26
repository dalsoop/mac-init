use std::process::Command;

use crate::common;

const DALCENTER_HOST: &str = "10.50.0.105";
const DALCENTER_PORTS: &[(&str, &str, u16)] = &[
    ("dalcenter", "dalcenter 자체 개발", 11192),
    ("veilkey", "VeilKey 개발", 11190),
    ("gaya", "가야의 연결점", 11191),
    ("veilkey-v2", "VeilKey v2", 11193),
];

fn dalcenter_url(repo: &str) -> String {
    for (name, _, port) in DALCENTER_PORTS {
        if *name == repo {
            return format!("http://{DALCENTER_HOST}:{port}");
        }
    }
    // 기본값
    format!("http://{DALCENTER_HOST}:{}", DALCENTER_PORTS[0].2)
}

fn dalcenter_bin() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/시스템/bin/dalcenter")
}

pub fn status() {
    println!("=== Dalcenter 상태 ===\n");

    // dalcenter 바이너리
    let bin = dalcenter_bin();
    let has_bin = std::path::Path::new(&bin).exists();
    println!("[바이너리] {}", if has_bin { "✓ 설치됨" } else { "✗ 미설치" });

    // 각 daemon 연결 확인
    println!("\n[Daemon]");
    for (name, desc, port) in DALCENTER_PORTS {
        let url = format!("http://{DALCENTER_HOST}:{port}");
        let (ok, _) = common::run_cmd_quiet("curl", &["-s", "--connect-timeout", "2", &format!("{url}/api/status")]);
        println!("  {name:12} ({desc}) :{port} {}", if ok { "✓" } else { "✗" });
    }

    // 실행 중인 dal 목록
    if has_bin {
        println!("\n[실행 중인 Dal]");
        for (name, _, port) in DALCENTER_PORTS {
            let url = format!("http://{DALCENTER_HOST}:{port}");
            let output = Command::new(&bin)
                .args(["ps"])
                .env("DALCENTER_URL", &url)
                .output();
            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().skip(1).collect();
                if !lines.is_empty() {
                    println!("  [{name}]");
                    for line in lines {
                        println!("    {line}");
                    }
                }
            }
        }
    }
}

pub fn ps(repo: &str) {
    let url = dalcenter_url(repo);
    let bin = dalcenter_bin();

    let _ = Command::new(&bin)
        .args(["ps"])
        .env("DALCENTER_URL", &url)
        .status();
}

pub fn wake(repo: &str, dal: &str) {
    let url = dalcenter_url(repo);
    let bin = dalcenter_bin();

    println!("[dal] {dal} 깨우는 중... ({repo} @ {url})");
    let _ = Command::new(&bin)
        .args(["wake", dal])
        .env("DALCENTER_URL", &url)
        .status();
}

pub fn sleep_dal(repo: &str, dal: &str) {
    let url = dalcenter_url(repo);
    let bin = dalcenter_bin();

    let _ = Command::new(&bin)
        .args(["sleep", dal])
        .env("DALCENTER_URL", &url)
        .status();
}

pub fn attach(repo: &str, dal: &str) {
    let url = dalcenter_url(repo);
    let bin = dalcenter_bin();

    let _ = Command::new(&bin)
        .args(["attach", dal])
        .env("DALCENTER_URL", &url)
        .status();
}

pub fn logs(repo: &str, dal: &str) {
    let url = dalcenter_url(repo);
    let bin = dalcenter_bin();

    let _ = Command::new(&bin)
        .args(["logs", dal])
        .env("DALCENTER_URL", &url)
        .status();
}
