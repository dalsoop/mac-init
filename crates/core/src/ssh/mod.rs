use std::process::Command;

use crate::common;
use crate::config::Config;

pub fn status() {
    println!("=== SSH 상태 ===\n");

    let cfg = Config::load();

    // SSH 키 확인
    let home = std::env::var("HOME").unwrap_or_default();
    let key_types = ["id_ed25519", "id_rsa", "id_ecdsa"];

    println!("[로컬 SSH 키]");
    for key in key_types {
        let path = format!("{home}/.ssh/{key}");
        let exists = std::path::Path::new(&path).exists();
        if exists {
            println!("  ✓ {path}");
        }
    }

    // Proxmox SSH 연결
    let (ok, _) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, "echo ok");
    println!(
        "\n[Proxmox SSH] {}@{}: {}",
        cfg.proxmox.user,
        cfg.proxmox.host,
        if ok {
            "✓ 연결 가능"
        } else {
            "✗ 연결 불가"
        }
    );

    // 로컬 root SSH 키
    let root_key = std::path::Path::new("/var/root/.ssh/id_ed25519").exists();
    println!(
        "\n[로컬 root SSH 키] {}",
        if root_key { "✓ 존재" } else { "✗ 없음" }
    );
}

pub fn copy_key(host: &str) {
    let cfg = Config::load();
    let target_host = if host.is_empty() {
        &cfg.proxmox.host
    } else {
        host
    };
    let user = &cfg.proxmox.user;

    let home = std::env::var("HOME").unwrap_or_default();
    let pub_key_path = format!("{home}/.ssh/id_ed25519.pub");

    if !std::path::Path::new(&pub_key_path).exists() {
        eprintln!("[ssh] SSH 키가 없습니다. 먼저 생성하세요:");
        eprintln!("  ssh-keygen -t ed25519");
        std::process::exit(1);
    }

    let pub_key = std::fs::read_to_string(&pub_key_path)
        .unwrap_or_default()
        .trim()
        .to_string();

    // sshpass로 키 복사 시도
    let (has_sshpass, _) = common::run_cmd_quiet("which", &["sshpass"]);

    if has_sshpass {
        let password = std::env::var("MOUNT_PASSWORD").unwrap_or_default();
        if password.is_empty() {
            eprintln!("[ssh] MOUNT_PASSWORD가 필요합니다. .env 파일을 확인하세요.");
            std::process::exit(1);
        }

        let remote_cmd = format!(
            "mkdir -p ~/.ssh && echo '{}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys",
            pub_key
        );

        println!("[ssh] {user}@{target_host}에 SSH 키 등록 중...");
        let output = Command::new("sshpass")
            .args([
                "-p",
                &password,
                "ssh",
                "-o",
                "StrictHostKeyChecking=accept-new",
                &format!("{user}@{target_host}"),
                &remote_cmd,
            ])
            .output()
            .expect("sshpass 실행 실패");

        if output.status.success() {
            println!("[ssh] SSH 키 등록 완료");
        } else {
            eprintln!(
                "[ssh] SSH 키 등록 실패: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        println!("[ssh] 아래 명령을 대상 서버에서 실행하세요:");
        println!("  echo '{}' >> ~/.ssh/authorized_keys", pub_key);
    }
}

pub fn test(host: &str) {
    let cfg = Config::load();
    let target_host = if host.is_empty() {
        &cfg.proxmox.host
    } else {
        host
    };
    let user = &cfg.proxmox.user;

    println!("[ssh] {}@{} 연결 테스트...", user, target_host);
    let (ok, output) = common::ssh_cmd(target_host, user, "hostname && uptime");
    if ok {
        println!("[ssh] 연결 성공:");
        print!("{output}");
    } else {
        println!("[ssh] 연결 실패");
    }
}
