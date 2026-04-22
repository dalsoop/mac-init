//! TUI v2 스펙 출력

use crate::config::load;
use crate::svn;
use mac_common::tui_spec::{self, TuiSpec};
use std::path::PathBuf;

pub fn print_spec() {
    let config = load();
    let installed = svn::installed();
    let server_ok = config.servers.iter().any(|s| svn::check_server(&s.url));
    let checked_out = config.repos.iter()
        .filter(|r| PathBuf::from(&r.local_path).join(".svn").exists())
        .count();

    let usage_summary: String = if !installed {
        "svn 미설치".into()
    } else if config.servers.is_empty() {
        "서버 미등록".into()
    } else if !server_ok {
        "서버 접속 불가".into()
    } else {
        format!("서버 {}, 레포 {}/{}", config.servers.len(), checked_out, config.repos.len())
    };

    let mut kv_items = vec![
        tui_spec::kv_item("SVN CLI",
            &if installed { format!("✓ v{}", svn::version()) } else { "✗ 미설치".into() },
            if installed { "ok" } else { "error" }),
        tui_spec::kv_item("서버", &format!("{} 개", config.servers.len()),
            if config.servers.is_empty() { "warn" } else { "ok" }),
        tui_spec::kv_item("계정", &format!("{} 개", config.accounts.len()),
            if config.accounts.is_empty() { "warn" } else { "ok" }),
        tui_spec::kv_item("레포", &format!("{}/{} 체크아웃", checked_out, config.repos.len()),
            if checked_out == config.repos.len() && !config.repos.is_empty() { "ok" } else { "warn" }),
    ];

    for r in &config.repos {
        let exists = PathBuf::from(&r.local_path).join(".svn").exists();
        kv_items.push(tui_spec::kv_item(
            &format!("  📁 {}", r.name),
            if exists { "✓" } else { "✗" },
            if exists { "ok" } else { "warn" },
        ));
    }

    TuiSpec::new("svn")
        .refresh(30)
        .usage(installed && server_ok, &usage_summary)
        .kv("상태", kv_items)
        .buttons()
        .print();
}
