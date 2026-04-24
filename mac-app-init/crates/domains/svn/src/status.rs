//! mai run svn status / test — 시스템 전체 현황 + 연결 테스트

use crate::config::{self, load};
use crate::svn;
use mac_common::cmd;
use std::path::PathBuf;
use std::process::Command;

pub fn status() {
    let cfg = load();

    println!("=== SVN 상태 ===\n");

    if svn::installed() {
        println!("[svn] ✓ v{}", svn::version());
    } else {
        println!("[svn] ✗ 미설치 → mai run svn install");
        return;
    }

    println!("\n[서버] {} 개", cfg.servers.len());
    for s in &cfg.servers {
        let ok = svn::check_server(&s.url);
        println!("  {} {} → {}", if ok { "✓" } else { "✗" }, s.name, s.url);
    }
    if cfg.servers.is_empty() {
        println!("  (없음) → mai run svn server add <name> --url <url>");
    }

    println!("\n[계정] {} 개", cfg.accounts.len());
    for a in &cfg.accounts {
        let has_pw = !cmd::stdout("mai", &["run", "env", "get-password", &a.env_card]).is_empty();
        println!("  {} {} (server: {}, user: {})",
            if has_pw { "✓" } else { "✗" }, a.name, a.server, a.username);
    }
    if cfg.accounts.is_empty() {
        println!("  (없음) → mai run svn account add ...");
    }

    println!("\n[레포] {} 개", cfg.repos.len());
    for r in &cfg.repos {
        let exists = PathBuf::from(&r.local_path).join(".svn").exists();
        let rev = if exists {
            let out = cmd::stdout("svn", &["info", "--show-item", "revision", &r.local_path]);
            if out.is_empty() { "?".into() } else { format!("r{}", out) }
        } else {
            "미체크아웃".into()
        };
        println!("  {} {} → {} ({})", if exists { "✓" } else { "✗" }, r.name, r.local_path, rev);
    }
    if cfg.repos.is_empty() {
        println!("  (없음) → mai run svn repo add <name>");
    }
}

pub fn test(server_name: Option<&str>, account_name: Option<&str>) {
    if !svn::installed() {
        println!("✗ svn 미설치 → mai run svn install");
        return;
    }

    let cfg = load();

    let srv = server_name
        .and_then(|n| cfg.server(n))
        .or_else(|| cfg.default_server());
    let srv = match srv {
        Some(s) => s.clone(),
        None => { println!("✗ 서버 없음 → mai run svn server add ..."); return; }
    };

    let acct = account_name
        .and_then(|n| cfg.account_for(n))
        .or_else(|| cfg.default_account(&srv.name));

    println!("=== SVN 연결 테스트: {} ===\n", srv.name);
    println!("서버: {}", srv.url);

    if svn::check_server(&srv.url) {
        println!("✓ 서버 접속 가능");
    } else {
        println!("✗ 서버 접속 불가 → VPN 확인");
        return;
    }

    if let Some(a) = acct {
        let pw = config::get_password(a);
        let test_url = cfg.repos.iter()
            .find(|r| r.server == srv.name)
            .and_then(|r| cfg.repo_url(r))
            .unwrap_or_else(|| srv.url.clone());

        let mut args = vec!["info", &test_url, "--username", &a.username, "--non-interactive"];
        let pw_str;
        if let Some(ref p) = pw {
            pw_str = p.clone();
            args.extend_from_slice(&["--password", &pw_str]);
        }

        match Command::new("svn").args(&args).output() {
            Ok(o) if o.status.success() => println!("✓ 인증 성공 (계정: {}, user: {})", a.name, a.username),
            _ => println!("✗ 인증 실패 (계정: {})", a.name),
        }

        println!("\n[레포 접근]");
        for r in cfg.repos.iter().filter(|r| r.server == srv.name) {
            if let Some(url) = cfg.repo_url(r) {
                let mut rargs = vec!["info", &url, "--username", &a.username, "--non-interactive"];
                let pw2;
                if let Some(ref p) = pw {
                    pw2 = p.clone();
                    rargs.extend_from_slice(&["--password", &pw2]);
                }
                match Command::new("svn").args(&rargs).output() {
                    Ok(o) if o.status.success() => println!("  ✓ {}", r.name),
                    _ => println!("  ✗ {}", r.name),
                }
            }
        }
    } else {
        println!("⚠ 계정 없음 — 인증 테스트 생략");
    }
}
