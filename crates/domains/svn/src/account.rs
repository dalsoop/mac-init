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

pub fn add(name: &str, username: &str, password: &str, server: Option<&str>) {
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

    let status = Command::new("mai")
        .args([
            "run", "env", "add", &env_card,
            "--host", &host, "--port", "80", "--scheme", "http",
            "--user", username, "--password", password,
            "--description", &format!("SVN 계정: {} ({})", name, srv),
        ])
        .status();

    if let Ok(s) = status {
        if !s.success() {
            let _ = Command::new("mai")
                .args(["run", "env", "set-password", &env_card, password])
                .status();
        }
    }

    config.accounts.push(config::Account {
        name: name.into(),
        server: srv.clone(),
        username: username.into(),
        env_card,
    });
    save(&config);
    println!("✓ 계정 추가: {} (server: {}, user: {})", name, srv, username);
}

pub fn rm(name: &str) {
    let mut config = load();
    let before = config.accounts.len();
    config.accounts.retain(|a| a.name != name);
    if config.accounts.len() == before {
        println!("✗ 찾을 수 없음: {}", name);
        return;
    }
    save(&config);
    println!("✓ 계정 삭제: {} (env 카드 수동 삭제: mai run env rm svn-{})", name, name);
}
