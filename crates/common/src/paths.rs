//! 공통 경로 헬퍼.
//! ~/.mac-app-init/ 하위 경로 통합.

use std::path::PathBuf;
use std::process::Command;

/// $HOME. 없으면 /tmp.
pub fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

/// ~/.mac-app-init/
pub fn base() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init")
}

/// ~/.mac-app-init/cards/
pub fn cards_dir() -> PathBuf {
    base().join("cards")
}

/// ~/.mac-app-init/domains/
pub fn domains_dir() -> PathBuf {
    base().join("domains")
}

/// ~/Library/LaunchAgents/
pub fn launch_agents_dir() -> PathBuf {
    PathBuf::from(home()).join("Library/LaunchAgents")
}

/// ~/Documents/WORK/
pub fn work_dir() -> PathBuf {
    PathBuf::from(home()).join("Documents/WORK")
}

/// `mai` 매니저 바이너리 경로.
pub fn manager_bin() -> PathBuf {
    let local = PathBuf::from(home()).join(".local/bin/mai");
    if local.exists() {
        return local;
    }

    let cargo = PathBuf::from(home()).join(".cargo/bin/mai");
    if cargo.exists() {
        return cargo;
    }

    if let Ok(output) = Command::new("which").arg("mai").output() {
        if output.status.success() {
            let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !resolved.is_empty() {
                return PathBuf::from(resolved);
            }
        }
    }

    PathBuf::from("mai")
}

/// ~ 확장. "~/foo" → "/Users/xxx/foo"
pub fn expand(p: &str) -> String {
    if p.starts_with('~') {
        p.replacen('~', &home(), 1)
    } else {
        p.to_string()
    }
}
