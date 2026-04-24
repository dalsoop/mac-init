//! 계정 카드 CRUD

use crate::config::{self, load, save};
use mac_common::cmd;
use std::process::Command;

pub fn list() {
    let config = load();
    if config.accounts.is_empty() {
        println!("등록된 계정 없음");
        println!("  → mai run svn account add admin --username admin --password <pw>");
        return;
    }
    println!("NAME           SERVER         USERNAME       AUTH");
    println!("──────────────────────────────────────────────────────");
    for a in &config.accounts {
        let has_pw = !cmd::stdout("mai", &["run", "env", "get-password", &a.env_card]).is_empty();
        println!("{:<15}{:<15}{:<15}{}", a.name, a.server, a.username, if has_pw { "✓" } else { "✗" });
    }
}

pub fn add(name: &str, username: &str, password: Option<&str>, server: Option<&str>) {
    let mut config = load();

    if config.accounts.iter().any(|a| a.name == name) {
        println!("✗ 이미 존재: {}", name);
        return;
    }

    let srv = server
        .map(String::from)
        .or_else(|| config.default_server().map(|s| s.name.clone()));
    let srv = match srv {
        Some(s) => s,
        None => {
            println!("✗ 서버가 없습니다. 먼저 등록:");
            println!("  → mai run svn server add <name> --url <url>");
            return;
        }
    };

    if config.server(&srv).is_none() {
        println!("✗ 서버 카드 없음: {}", srv);
        return;
    }

    let env_card = format!("svn-{}", name);
    let host = config.server(&srv)
        .map(|s| s.url.trim_start_matches("http://").trim_start_matches("https://").split('/').next().unwrap_or(""))
        .unwrap_or("")
        .to_string();

    // Create env card (with or without password)
    let mut env_args = vec![
        "run", "env", "add", &env_card,
        "--host", &host, "--port", "80", "--scheme", "http",
        "--user", username,
        "--description",
    ];
    let desc = format!("SVN 계정: {} ({})", name, srv);
    env_args.push(&desc);

    if let Some(pw) = password {
        env_args.extend_from_slice(&["--password", pw]);
    }

    let env_ok = Command::new("mai")
        .args(&env_args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !env_ok {
        // Card might exist — try set-password if password provided
        if let Some(pw) = password {
            let set_ok = Command::new("mai")
                .args(["run", "env", "set-password", &env_card, pw])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !set_ok {
                eprintln!("✗ env 카드 생성 실패: {}", env_card);
                return;
            }
        } else {
            eprintln!("✗ env 카드 생성 실패: {}", env_card);
            return;
        }
    }

    config.accounts.push(config::Account {
        name: name.into(),
        server: srv.clone(),
        username: username.into(),
        env_card: env_card.clone(),
    });
    save(&config);
    let pw_hint = if password.is_some() { "" } else { " (비밀번호: mai run env set-password {env_card} <pw>)" };
    println!("✓ 계정 추가: {} (server: {}, user: {}){}", name, srv, username, pw_hint);
}

pub fn rm(name: &str) {
    let mut config = load();
    let card = config.accounts.iter().find(|a| a.name == name).cloned();
    let before = config.accounts.len();
    config.accounts.retain(|a| a.name != name);
    if config.accounts.len() == before {
        println!("✗ 찾을 수 없음: {}", name);
        return;
    }
    save(&config);

    // Attempt env card cleanup
    if let Some(c) = card {
        let _ = Command::new("mai")
            .args(["run", "env", "rm", &c.env_card])
            .status();
    }
    println!("✓ 계정 삭제: {} (env 카드도 정리됨)", name);
}
