use std::process::Command;

use crate::common;

pub fn status() {
    println!("=== GitHub 상태 ===\n");

    // gh CLI 설치 확인
    let (has_gh, _) = common::run_cmd_quiet("which", &["gh"]);
    println!(
        "[gh CLI] {}",
        if has_gh {
            "✓ 설치됨"
        } else {
            "✗ 미설치"
        }
    );

    if !has_gh {
        println!("  mai github install");
        return;
    }

    // 버전
    let (_, ver) = common::run_cmd_quiet("gh", &["--version"]);
    if let Some(line) = ver.lines().next() {
        println!("[버전] {}", line.trim());
    }

    // 인증 상태
    let (auth_ok, auth) = common::run_cmd_quiet("gh", &["auth", "status"]);
    if auth_ok {
        for line in auth.lines() {
            let trimmed = line.trim();
            if trimmed.contains("Logged in") || trimmed.contains("account") {
                println!("[인증] ✓ {trimmed}");
            }
            if trimmed.contains("Token") {
                println!("[토큰] ✓ {trimmed}");
            }
        }
    } else {
        // gh auth status는 stderr로 출력
        let output = Command::new("gh")
            .args(["auth", "status"])
            .output()
            .unwrap_or_else(|e| panic!("gh 실행 실패: {e}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            let trimmed = line.trim();
            if trimmed.contains("Logged in") || trimmed.contains("account") {
                println!("[인증] ✓ {trimmed}");
            } else if trimmed.contains("Token") {
                println!("[토큰] {trimmed}");
            } else if trimmed.contains("not logged") {
                println!("[인증] ✗ 미인증");
            }
        }
    }

    // Git 설정
    let (_, name) = common::run_cmd_quiet("git", &["config", "--global", "user.name"]);
    let (_, email) = common::run_cmd_quiet("git", &["config", "--global", "user.email"]);
    println!("\n[git config]");
    println!(
        "  user.name: {}",
        if name.trim().is_empty() {
            "✗ 미설정"
        } else {
            name.trim()
        }
    );
    println!(
        "  user.email: {}",
        if email.trim().is_empty() {
            "✗ 미설정"
        } else {
            email.trim()
        }
    );

    // SSH 키 등록 확인
    let (ssh_ok, ssh_keys) = common::run_cmd_quiet("gh", &["ssh-key", "list"]);
    if ssh_ok {
        let count = ssh_keys.lines().count();
        println!("\n[SSH 키] {}개 등록됨", count);
    }
}

pub fn install() {
    // gh CLI
    let (has_gh, _) = common::run_cmd_quiet("which", &["gh"]);
    if has_gh {
        println!("[github] gh CLI 이미 설치됨");
    } else {
        println!("[github] gh CLI 설치 중...");
        let ok = Command::new("brew")
            .args(["install", "gh"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            println!("[github] gh CLI 설치 완료");
        } else {
            eprintln!("[github] gh CLI 설치 실패");
            std::process::exit(1);
        }
    }

    // 인증 확인
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .unwrap_or_else(|e| panic!("gh 실행 실패: {e}"));
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stderr.contains("Logged in") {
        println!("[github] 이미 인증됨");
    } else {
        println!("[github] GitHub 인증 시작...");
        let _ = Command::new("gh")
            .args(["auth", "login", "--web", "--git-protocol", "https"])
            .status();
    }
}

pub fn auth() {
    println!("[github] GitHub 인증 시작...");
    let _ = Command::new("gh")
        .args(["auth", "login", "--web", "--git-protocol", "https"])
        .status();
}

pub fn setup_git(name: &str, email: &str) {
    println!("[github] Git 설정 중...");

    let (ok1, _, _) = common::run_cmd("git", &["config", "--global", "user.name", name]);
    let (ok2, _, _) = common::run_cmd("git", &["config", "--global", "user.email", email]);

    if ok1 && ok2 {
        println!("[github] Git 설정 완료:");
        println!("  user.name: {name}");
        println!("  user.email: {email}");
    }
}

pub fn setup_ssh() {
    let home = std::env::var("HOME").unwrap_or_default();
    let key_path = format!("{home}/.ssh/id_ed25519.pub");

    if !std::path::Path::new(&key_path).exists() {
        eprintln!("[github] SSH 키가 없습니다.");
        eprintln!("  ssh-keygen -t ed25519");
        std::process::exit(1);
    }

    let pub_key = std::fs::read_to_string(&key_path).unwrap_or_default();
    println!("[github] GitHub에 SSH 키 등록 중...");

    let hostname = Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "mac".to_string());

    let (ok, _, _) = common::run_cmd("gh", &["ssh-key", "add", &key_path, "--title", &hostname]);
    if ok {
        println!(
            "[github] SSH 키 등록 완료: {}",
            pub_key.trim().split(' ').last().unwrap_or("")
        );
    }
}

pub fn repos() {
    let (_, output) = common::run_cmd_quiet(
        "gh",
        &[
            "repo",
            "list",
            "--limit",
            "30",
            "--json",
            "name,description,visibility,updatedAt",
            "--template",
            "{{range .}}{{.name}}\t{{.visibility}}\t{{.description}}\n{{end}}",
        ],
    );
    println!("=== GitHub 레포 목록 ===\n");
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() >= 2 {
            let vis = if parts[1] == "PUBLIC" {
                ""
            } else {
                " (private)"
            };
            let desc = if parts.len() >= 3 && !parts[2].is_empty() {
                format!(" — {}", parts[2])
            } else {
                String::new()
            };
            println!("  {}{vis}{desc}", parts[0]);
        }
    }
}
