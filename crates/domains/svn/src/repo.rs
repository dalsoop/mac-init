//! 레포 카드 CRUD + checkout/update

use crate::config::{self, get_password, load, save, svn_root};
use crate::svn;
use mac_common::cmd;
use std::path::PathBuf;
use std::process::Command;

pub fn list() {
    let config = load();
    if config.repos.is_empty() {
        println!("등록된 레포 없음");
        println!("  → mai run svn repo add game");
        return;
    }
    println!("NAME           SERVER         ACCOUNT        STATUS");
    println!("               URL");
    println!("               PATH");
    println!("──────────────────────────────────────────────────────────");
    for r in &config.repos {
        let exists = PathBuf::from(&r.local_path).join(".svn").exists();
        let rev = if exists {
            let out = cmd::stdout("svn", &["info", "--show-item", "revision", &r.local_path]);
            if out.is_empty() { "?".into() } else { format!("r{}", out) }
        } else {
            "미체크아웃".into()
        };
        let url = config.repo_url(r).unwrap_or_else(|| "?".into());
        println!("{:<15}{:<15}{:<15}{} {}",
            r.name, r.server, r.account,
            if exists { "✓" } else { "✗" }, rev);
        println!("               {}", url);
        println!("               {}", r.local_path);
    }
}

pub fn add(name: &str, svn_path: Option<&str>, local_path: Option<&str>, server: Option<&str>, account: Option<&str>) {
    let mut config = load();

    if config.repos.iter().any(|r| r.name == name) {
        println!("✗ 이미 존재: {}", name);
        return;
    }

    let srv = server.map(String::from)
        .or_else(|| config.default_server().map(|s| s.name.clone()));
    let srv = match srv {
        Some(s) => s,
        None => { println!("✗ 서버가 없습니다 → mai run svn server add ..."); return; }
    };

    let acct = account.map(String::from)
        .or_else(|| config.default_account(&srv).map(|a| a.name.clone()));
    let acct = match acct {
        Some(a) => a,
        None => { println!("✗ 계정이 없습니다 → mai run svn account add ..."); return; }
    };

    let default_svn = format!("{}/trunk", name);
    let svn_path = svn_path.unwrap_or(&default_svn).to_string();
    let local = local_path.map(String::from)
        .unwrap_or_else(|| svn_root().join(name).to_string_lossy().to_string());
    let full_url = config.server(&srv)
        .map(|s| format!("{}/{}", s.url, svn_path))
        .unwrap_or_else(|| format!("?/{}", svn_path));

    config.repos.push(config::Repo {
        name: name.into(), server: srv.clone(), account: acct.clone(),
        svn_path, local_path: local.clone(),
    });
    save(&config);
    println!("✓ 레포 추가: {} (server: {}, 계정: {})", name, srv, acct);
    println!("  URL: {}", full_url);
    println!("  경로: {}", local);
}

pub fn rm(name: &str) {
    let mut config = load();
    let before = config.repos.len();
    config.repos.retain(|r| r.name != name);
    if config.repos.len() == before {
        println!("✗ 찾을 수 없음: {}", name);
        return;
    }
    save(&config);
    println!("✓ 레포 삭제: {} (로컬 파일 유지)", name);
}

pub fn checkout(name: &str) {
    if !svn::installed() { println!("✗ svn 미설치 → mai run svn install"); return; }

    let config = load();
    let repo = match config.repos.iter().find(|r| r.name == name) {
        Some(r) => r.clone(),
        None => { println!("✗ 레포 카드 없음: {} → mai run svn repo add {}", name, name); return; }
    };

    if PathBuf::from(&repo.local_path).join(".svn").exists() {
        println!("✓ 이미 체크아웃됨: {}", repo.local_path);
        println!("  → mai run svn update {}", name);
        return;
    }

    let url = match config.repo_url(&repo) {
        Some(u) => u,
        None => { println!("✗ 서버 카드 없음: {}", repo.server); return; }
    };
    let account = match config.account_for(&repo.account) {
        Some(a) => a.clone(),
        None => { println!("✗ 계정 카드 없음: {}", repo.account); return; }
    };

    if let Some(parent) = PathBuf::from(&repo.local_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let password = get_password(&account);
    println!("체크아웃: {} → {}\n", url, repo.local_path);

    let mut args = vec!["checkout".into(), url, repo.local_path.clone(),
                        "--username".into(), account.username.clone(), "--non-interactive".into()];
    if let Some(pw) = password { args.extend(["--password".into(), pw]); }

    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match Command::new("svn").args(&str_args).status() {
        Ok(s) if s.success() => println!("\n✓ 체크아웃 완료: {}", repo.local_path),
        _ => println!("\n✗ 체크아웃 실패"),
    }
}

pub fn update(name: Option<&str>) {
    if !svn::installed() { println!("✗ svn 미설치 → mai run svn install"); return; }

    let config = load();
    let repos: Vec<&config::Repo> = if let Some(n) = name {
        match config.repos.iter().find(|r| r.name == n) {
            Some(r) => vec![r],
            None => { println!("✗ 레포 카드 없음: {}", n); return; }
        }
    } else {
        config.repos.iter().collect()
    };

    if repos.is_empty() { println!("등록된 레포 없음"); return; }

    for repo in repos {
        if !PathBuf::from(&repo.local_path).join(".svn").exists() {
            println!("⏭ {} — 미체크아웃 → mai run svn checkout {}", repo.name, repo.name);
            continue;
        }
        print!("{} ... ", repo.name);
        match Command::new("svn").args(["update", &repo.local_path]).output() {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let rev = stdout.lines().last().unwrap_or("").trim();
                println!("✓ {}", rev);
            }
            _ => println!("✗ 실패"),
        }
    }
}

pub fn open(name: &str) {
    let config = load();
    let repo = match config.repos.iter().find(|r| r.name == name) {
        Some(r) => r,
        None => { println!("✗ 레포 카드 없음: {}", name); return; }
    };
    let url = match config.repo_url(repo) {
        Some(u) => u,
        None => { println!("✗ 서버 카드 없음: {}", repo.server); return; }
    };
    println!("브라우저: {}", url);
    let _ = std::process::Command::new("open").arg(&url).status();
}
