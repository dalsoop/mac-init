//! 서버 카드 CRUD

use crate::config::{self, load, save};
use crate::protect;
use crate::svn;

pub fn list() {
    let config = load();
    if config.servers.is_empty() {
        println!("등록된 서버 없음");
        println!("  → mai run svn server add <name> --url <url>");
        return;
    }
    println!("NAME           URL                                    STATUS");
    println!("──────────────────────────────────────────────────────────────");
    for s in &config.servers {
        let ok = svn::check_server(&s.url);
        println!("{:<15}{:<40} {}", s.name, s.url, if ok { "✓" } else { "✗" });
    }
}

pub fn add(name: &str, url: &str) {
    let mut config = load();
    if config.servers.iter().any(|s| s.name == name) {
        println!("✗ 이미 존재: {}", name);
        return;
    }
    let url = url.trim_end_matches('/').to_string();
    config.servers.push(config::Server { name: name.into(), url: url.clone() });
    save(&config);
    protect::ensure();
    println!("✓ 서버 추가: {} → {}", name, url);
}

pub fn rm(name: &str) {
    let mut config = load();
    let before = config.servers.len();
    config.servers.retain(|s| s.name != name);
    if config.servers.len() == before {
        println!("✗ 찾을 수 없음: {}", name);
        return;
    }
    save(&config);
    println!("✓ 서버 삭제: {}", name);
}
