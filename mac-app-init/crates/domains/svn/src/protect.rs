//! mount-protect.json 인터페이스 — mount 도메인과의 의존성 계약

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MOUNT_PROTECT: &str = ".mac-app-init/mount-protect.json";
const PROTECT_KEY: &str = "SVN";

#[derive(Serialize, Deserialize, Default)]
struct Protect {
    #[serde(default)]
    protected: Vec<String>,
}

/// mount 도메인의 sweep_nas_orphans 에서 SVN 경로를 건드리지 않도록 등록.
pub fn ensure() {
    let path = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(MOUNT_PROTECT);

    let mut protect: Protect = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Protect::default()
    };

    if !protect.protected.iter().any(|p| p == PROTECT_KEY) {
        protect.protected.push(PROTECT_KEY.to_string());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string_pretty(&protect).unwrap_or_default();
        std::fs::write(&path, json).ok();
    }
}
