//! 설치된 도메인 발견 + tui-spec 호출

use crate::spec::DomainSpec;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Registry trait — 외부 의존(프로세스, 파일시스템) 추상화.
/// 프로덕션은 SystemRegistry, 테스트는 MockRegistry.
pub trait Registry: Send + Sync + 'static {
    fn installed_domains(&self) -> Vec<String>;
    fn available_domains(&self) -> Vec<String>;
    fn card_inventory(&self) -> Vec<(String, bool)> {
        Vec::new()
    }
    fn fetch_spec(&self, domain: &str) -> Option<DomainSpec>;
    fn run_action(&self, domain: &str, command: &str, args: &[String]) -> String;
    fn install_domain(&self, name: &str) -> String;
    fn remove_domain(&self, name: &str) -> String;
}

// ── 프로덕션 구현 ──

pub struct SystemRegistry;

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn domains_dir() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/domains")
}

fn manager_bin() -> PathBuf {
    let local = PathBuf::from(home()).join(".local/bin/mai");
    if local.exists() {
        return local;
    }
    PathBuf::from("mai")
}

fn domain_bin(name: &str) -> PathBuf {
    domains_dir().join(format!("mac-domain-{}", name))
}

impl Registry for SystemRegistry {
    fn installed_domains(&self) -> Vec<String> {
        let registry = domains_dir().join("registry.json");
        if !registry.exists() {
            return Vec::new();
        }
        let content = fs::read_to_string(&registry).unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        json.get("installed")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| d.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn available_domains(&self) -> Vec<String> {
        let output = Command::new(manager_bin()).arg("available").output();
        let Ok(o) = output else {
            return Vec::new();
        };
        let stdout = String::from_utf8_lossy(&o.stdout);
        stdout
            .lines()
            .skip(2)
            .filter_map(|line| line.split_whitespace().next().map(String::from))
            .filter(|s| !s.is_empty() && !s.starts_with('─'))
            .collect()
    }

    fn card_inventory(&self) -> Vec<(String, bool)> {
        let output = Command::new(manager_bin())
            .args(["card", "list", "--all"])
            .output();
        let Ok(o) = output else {
            return Vec::new();
        };
        let stdout = String::from_utf8_lossy(&o.stdout);
        stdout
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let state = parts.next()?;
                let name = parts.next()?;
                Some((name.to_string(), state == "enabled"))
            })
            .collect()
    }

    fn fetch_spec(&self, domain: &str) -> Option<DomainSpec> {
        let bin = domain_bin(domain);
        if !bin.exists() {
            return None;
        }
        let output = Command::new(&bin).arg("tui-spec").output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).ok()
    }

    fn run_action(&self, domain: &str, command: &str, args: &[String]) -> String {
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

    fn install_domain(&self, name: &str) -> String {
        let output = Command::new(manager_bin()).args(["install", name]).output();
        match output {
            Ok(o) => format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => format!("Error: {}", e),
        }
    }

    fn remove_domain(&self, name: &str) -> String {
        let output = Command::new(manager_bin()).args(["remove", name]).output();
        match output {
            Ok(o) => format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => format!("Error: {}", e),
        }
    }
}

// ── 하위 호환: 기존 free function → SystemRegistry 위임 ──

pub fn installed_domains() -> Vec<String> {
    SystemRegistry.installed_domains()
}
pub fn available_domains() -> Vec<String> {
    SystemRegistry.available_domains()
}
pub fn fetch_spec(domain: &str) -> Option<DomainSpec> {
    SystemRegistry.fetch_spec(domain)
}
pub fn run_action(domain: &str, command: &str, args: &[String]) -> String {
    SystemRegistry.run_action(domain, command, args)
}
pub fn install_domain(name: &str) -> String {
    SystemRegistry.install_domain(name)
}
pub fn remove_domain(name: &str) -> String {
    SystemRegistry.remove_domain(name)
}
