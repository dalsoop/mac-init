//! 공통 경로 헬퍼.
//! ~/.mac-app-init/ 하위 경로 통합.

use std::path::PathBuf;

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

/// ~ 확장. "~/foo" → "/Users/xxx/foo"
pub fn expand(p: &str) -> String {
    if p.starts_with('~') {
        p.replacen('~', &home(), 1)
    } else {
        p.to_string()
    }
}
