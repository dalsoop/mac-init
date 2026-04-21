//! 공통 경로 헬퍼.
//! ~/.mac-app-init/ 하위 경로 통합.

use std::path::PathBuf;
use std::process::Command;
use std::{fs, path::Path};

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

/// ~/.mac-app-init/config-source
pub fn config_source_path() -> PathBuf {
    base().join("config-source")
}

fn cwd_portable_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let root = cwd.join("portable/mai");
    if root.exists() && root.is_dir() {
        Some(root)
    } else {
        None
    }
}

fn configured_portable_root() -> Option<PathBuf> {
    let path = config_source_path();
    let content = fs::read_to_string(path).ok()?;
    let root = PathBuf::from(content.trim());
    if root.exists() && root.is_dir() {
        Some(root)
    } else {
        None
    }
}

fn managed_clone_portable_root() -> Option<PathBuf> {
    let root = base().join("src/mac-app-init/portable/mai");
    if root.exists() && root.is_dir() {
        Some(root)
    } else {
        None
    }
}

/// repo tracked 설정 원본. 있으면 SSOT 로 사용.
pub fn portable_root() -> Option<PathBuf> {
    configured_portable_root()
        .or_else(cwd_portable_root)
        .or_else(managed_clone_portable_root)
}

pub fn record_config_source(root: &Path) -> std::io::Result<()> {
    if let Some(parent) = config_source_path().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(config_source_path(), root.display().to_string())
}

/// 카드 SSOT 디렉터리. portable/mai/cards 가 있으면 그쪽이 원본.
pub fn ssot_cards_dir() -> PathBuf {
    portable_root()
        .map(|root| root.join("cards"))
        .unwrap_or_else(cards_dir)
}

pub fn ssot_mount_config_path() -> PathBuf {
    portable_root()
        .map(|root| root.join("mount.json"))
        .unwrap_or_else(|| base().join("mount.json"))
}

pub fn ssot_proxmox_bind_mounts_path() -> PathBuf {
    portable_root()
        .map(|root| root.join("proxmox-bind-mounts.json"))
        .unwrap_or_else(|| base().join("proxmox-bind-mounts.json"))
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
