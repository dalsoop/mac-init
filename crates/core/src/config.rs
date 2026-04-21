use serde::Deserialize;
use std::fs;

use crate::common;

#[derive(Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub proxmox: ProxmoxConfig,
    #[serde(default)]
    pub synology: NasConfig,
    #[serde(default)]
    pub truenas: NasConfig,
    #[serde(default)]
    pub mount: MountConfig,
}

#[derive(Deserialize, Clone)]
pub struct ProxmoxConfig {
    #[serde(default = "default_proxmox_host")]
    pub host: String,
    #[serde(default = "default_proxmox_user")]
    pub user: String,
    #[serde(default = "default_proxmox_port")]
    pub port: u16,
}

impl Default for ProxmoxConfig {
    fn default() -> Self {
        Self {
            host: default_proxmox_host(),
            user: default_proxmox_user(),
            port: default_proxmox_port(),
        }
    }
}

fn default_proxmox_host() -> String {
    std::env::var("PROXMOX_HOST").unwrap_or_else(|_| "192.168.2.50".to_string())
}
fn default_proxmox_user() -> String {
    std::env::var("PROXMOX_USER").unwrap_or_else(|_| "root".to_string())
}
fn default_proxmox_port() -> u16 {
    22
}

#[derive(Deserialize, Default, Clone)]
pub struct NasConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub user: String,
}

#[derive(Deserialize, Default, Clone)]
pub struct MountConfig {
    #[serde(default = "default_mount_base")]
    pub base_path: String,
    #[serde(default)]
    pub targets: Vec<MountTarget>,
}

fn default_mount_base() -> String {
    "/Volumes".to_string()
}

#[derive(Deserialize, Default, Clone)]
pub struct MountTarget {
    pub name: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub user: String,
    pub remote_path: String,
    #[serde(default)]
    pub mount_point: String,
    #[serde(default = "default_method")]
    pub method: String,
}

fn default_method() -> String {
    "sshfs".to_string()
}

impl Config {
    pub fn load() -> Self {
        let path = common::config_file();
        if !path.exists() {
            eprintln!("[config] {} 없음, 기본값 사용", path.display());
            return Config::default();
        }

        let content = fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("[config] 읽기 실패: {e}");
            String::new()
        });

        toml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("[config] 파싱 실패: {e}");
            Config::default()
        })
    }

    pub fn init() {
        let dir = common::config_dir();
        common::ensure_dir(&dir);

        // .env
        let env_path = common::env_file();
        if !env_path.exists() {
            let template = include_str!("../env.template");
            fs::write(&env_path, template).expect(".env 파일 생성 실패");
            println!("[config] {} 생성 완료", env_path.display());
        } else {
            println!("[config] {} 이미 존재", env_path.display());
        }

        // config.toml
        let cfg_path = common::config_file();
        if cfg_path.exists() {
            println!("[config] {} 이미 존재", cfg_path.display());
            return;
        }

        let template = r#"# mai 설정 파일
# 비밀번호/토큰은 .env 파일에서 관리합니다

[proxmox]
host = "192.168.2.50"
user = "root"
port = 22

[synology]
host = "192.168.2.15"
user = "proxmox"

[truenas]
host = "192.168.2.5"
user = "root"

[mount]
base_path = "/Volumes"

# --- Proxmox ---
[[mount.targets]]
name = "proxmox"
remote_path = "/"
method = "sshfs"

# --- Synology 전체 (Proxmox /mnt/synology 경유) ---
[[mount.targets]]
name = "synology"
remote_path = "/mnt/synology-organized"
method = "sshfs"

# --- TrueNAS 전체 (Proxmox /mnt/truenas 경유) ---
[[mount.targets]]
name = "truenas"
remote_path = "/mnt/truenas"
method = "sshfs"
"#;

        fs::write(&cfg_path, template).expect("설정 파일 생성 실패");
        println!("[config] {} 생성 완료", cfg_path.display());
        println!("  편집: nano {}", cfg_path.display());
    }

    pub fn status() {
        println!("=== 설정 상태 ===\n");

        let env_path = common::env_file();
        let env_exists = env_path.exists();
        println!(
            "[.env] {}: {}",
            env_path.display(),
            if env_exists { "✓" } else { "✗" }
        );

        if env_exists {
            let env_vars = [
                "PROXMOX_HOST",
                "PROXMOX_USER",
                "MOUNT_USER",
                "MOUNT_PASSWORD",
                "SYNOLOGY_PASSWORD",
                "TRUENAS_PASSWORD",
            ];
            for var in env_vars {
                let val = std::env::var(var).unwrap_or_default();
                let mark = if val.is_empty() { "✗" } else { "✓" };
                println!("  {mark} {var}");
            }
        }

        let cfg_path = common::config_file();
        let exists = cfg_path.exists();
        println!(
            "\n[config] {}: {}",
            cfg_path.display(),
            if exists {
                "✓"
            } else {
                "✗ (config init 실행 필요)"
            }
        );

        if !exists {
            return;
        }

        let cfg = Config::load();

        println!("\n[proxmox]");
        println!("  host: {}", cfg.proxmox.host);
        println!("  user: {}", cfg.proxmox.user);
        println!("  port: {}", cfg.proxmox.port);

        println!("\n[synology]");
        if cfg.synology.host.is_empty() {
            println!("  ✗ 미설정");
        } else {
            println!("  host: {}", cfg.synology.host);
            println!("  user: {}", cfg.synology.user);
            let pw = std::env::var("SYNOLOGY_PASSWORD").unwrap_or_default();
            println!(
                "  password: {}",
                if pw.is_empty() {
                    "✗"
                } else {
                    "✓ (설정됨)"
                }
            );
        }

        println!("\n[truenas]");
        if cfg.truenas.host.is_empty() {
            println!("  ✗ 미설정");
        } else {
            println!("  host: {}", cfg.truenas.host);
            println!("  user: {}", cfg.truenas.user);
            let pw = std::env::var("TRUENAS_PASSWORD").unwrap_or_default();
            println!(
                "  password: {}",
                if pw.is_empty() {
                    "✗"
                } else {
                    "✓ (설정됨)"
                }
            );
        }

        println!("\n[mount]");
        println!("  base_path: {}", cfg.mount.base_path);
        println!("  targets: {}개", cfg.mount.targets.len());
        for t in &cfg.mount.targets {
            let host = if t.host.is_empty() {
                &cfg.proxmox.host
            } else {
                &t.host
            };
            let user = if t.user.is_empty() {
                &cfg.proxmox.user
            } else {
                &t.user
            };
            let mp = if t.mount_point.is_empty() {
                format!("{}/{}", cfg.mount.base_path, t.name)
            } else {
                t.mount_point.clone()
            };
            println!(
                "    [{}] {user}@{host}:{} -> {} ({})",
                t.name, t.remote_path, mp, t.method
            );
        }
    }
}
