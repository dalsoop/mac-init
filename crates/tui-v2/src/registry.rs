//! 설치된 도메인 발견 + tui-spec 호출

use crate::spec::DomainSpec;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn home() -> String { std::env::var("HOME").unwrap_or_default() }

fn domains_dir() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/domains")
}

pub fn installed_domains() -> Vec<String> {
    let registry = domains_dir().join("registry.json");
    if !registry.exists() { return Vec::new(); }
    let content = fs::read_to_string(&registry).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    json.get("installed").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|d| d.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
        .unwrap_or_default()
}

fn mac_bin() -> &'static str { "mac" }

/// `mac available` 파싱 — 전체 도메인 이름 리스트
pub fn available_domains() -> Vec<String> {
    let output = Command::new(mac_bin()).arg("available").output();
    let Ok(o) = output else { return Vec::new(); };
    let stdout = String::from_utf8_lossy(&o.stdout);
    stdout.lines()
        .skip(2)
        .filter_map(|line| line.split_whitespace().next().map(String::from))
        .filter(|s| !s.is_empty() && !s.starts_with('─'))
        .collect()
}

pub fn install_domain(name: &str) -> String {
    let output = Command::new(mac_bin()).args(["install", name]).output();
    match output {
        Ok(o) => format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
        Err(e) => format!("Error: {}", e),
    }
}

pub fn remove_domain(name: &str) -> String {
    let output = Command::new(mac_bin()).args(["remove", name]).output();
    match output {
        Ok(o) => format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
        Err(e) => format!("Error: {}", e),
    }
}

pub fn domain_bin(name: &str) -> PathBuf {
    domains_dir().join(format!("mac-domain-{}", name))
}

/// 도메인 바이너리에서 tui-spec 받아오기
pub fn fetch_spec(domain: &str) -> Option<DomainSpec> {
    let bin = domain_bin(domain);
    if !bin.exists() { return None; }
    let output = Command::new(&bin).arg("tui-spec").output().ok()?;
    if !output.status.success() { return None; }
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).ok()
}

/// 도메인 명령 실행
pub fn run_action(domain: &str, command: &str, args: &[String]) -> String {
    let bin = domain_bin(domain);
    let mut cmd_args = vec![command.to_string()];
    cmd_args.extend(args.iter().cloned());
    let output = Command::new(&bin).args(&cmd_args).output();
    match output {
        Ok(o) => format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => format!("Error: {}", e),
    }
}
