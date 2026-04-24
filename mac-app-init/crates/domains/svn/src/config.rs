//! 데이터 구조체 + load/save — 순수 상태 관리만

use mac_common::cmd;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const WORK_MOUNT: &str = "Documents/WORK/MOUNT/SVN";

// ── 카드 구조체 ──

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SvnConfig {
    #[serde(default)]
    pub servers: Vec<Server>,
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub repos: Vec<Repo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub name: String,
    pub server: String,
    pub username: String,
    pub env_card: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repo {
    pub name: String,
    pub server: String,
    pub account: String,
    pub svn_path: String,
    pub local_path: String,
}

// ── Config 조회 ──

impl SvnConfig {
    pub fn server(&self, name: &str) -> Option<&Server> {
        self.servers.iter().find(|s| s.name == name)
    }

    pub fn default_server(&self) -> Option<&Server> {
        self.servers.first()
    }

    pub fn account_for(&self, name: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.name == name)
    }

    pub fn default_account(&self, server: &str) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|a| a.server == server)
            .or_else(|| self.accounts.first())
    }

    pub fn repo_url(&self, repo: &Repo) -> Option<String> {
        self.server(&repo.server)
            .map(|s| format!("{}/{}", s.url, repo.svn_path))
    }
}

// ── 영속화 ──

fn config_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".mac-app-init/svn.json")
}

pub fn load() -> SvnConfig {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        SvnConfig::default()
    }
}

pub fn save(config: &SvnConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(config).unwrap_or_default();
    if let Err(e) = std::fs::write(&path, &json) {
        eprintln!("✗ svn.json 저장 실패: {}", e);
    }
}

pub fn svn_root() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(WORK_MOUNT)
}

// ── 인증 헬퍼 ──

pub fn get_password(account: &Account) -> Option<String> {
    let pw = cmd::stdout("mai", &["run", "env", "get-password", &account.env_card]);
    if pw.is_empty() { None } else { Some(pw) }
}
