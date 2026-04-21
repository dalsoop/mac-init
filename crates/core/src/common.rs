use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

pub fn home_path(relative: &str) -> PathBuf {
    PathBuf::from(home()).join(relative)
}

/// Get required env var — exits with message if missing
pub fn env_required(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        eprintln!("[error] 환경변수 {key} 가 설정되지 않았습니다.");
        eprintln!("  ~/.env 파일을 확인하세요.");
        std::process::exit(1);
    })
}

/// Get optional env var
pub fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Get env var with fallback
pub fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Get dir from env var (HOME-relative)
pub fn env_dir(key: &str) -> PathBuf {
    let relative = env_required(key);
    home_path(&relative)
}

/// Get dir from env var with fallback
pub fn env_dir_or(key: &str, default: &str) -> PathBuf {
    let relative = env_or(key, default);
    home_path(&relative)
}

pub fn config_dir() -> PathBuf {
    home_path(".mac-app-init")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn env_file() -> PathBuf {
    home_path(".env")
}

pub fn manager_bin() -> PathBuf {
    let local = home_path(".local/bin/mai");
    if local.exists() {
        return local;
    }

    let cargo = home_path(".cargo/bin/mai");
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

/// Load ~/.env via dotenvx (handles encrypted values)
/// Falls back to plain text parsing if dotenvx not available
pub fn load_env() {
    let env_path = home_path(".env");
    if !env_path.exists() {
        return;
    }

    // Try dotenvx first (handles encrypted values)
    if load_env_dotenvx(&env_path) {
        return;
    }

    // Fallback: plain text (skips encrypted values)
    load_env_plain(&env_path);
}

fn load_env_dotenvx(path: &Path) -> bool {
    let output = Command::new("dotenvx")
        .args(["get", "-f", &path.to_string_lossy(), "--format", "json"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&stdout) {
                for (key, value) in &map {
                    if std::env::var(key).is_err() {
                        if let Some(val) = value.as_str() {
                            unsafe { std::env::set_var(key, val); }
                        }
                    }
                }
                return true;
            }
            // dotenvx get --format json not supported, try line-by-line
            false
        }
        _ => false,
    }
}

fn load_env_plain(path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            if std::env::var(key).is_err() && !value.is_empty() && !value.starts_with("encrypted:") {
                unsafe { std::env::set_var(key, value); }
            }
        }
    }
}

pub fn run_cmd(cmd: &str, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(cmd).args(args).output().unwrap_or_else(|e| {
        panic!("{cmd} 실행 실패: {e}");
    });
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        eprintln!("[error] {cmd} 실패: {stderr}");
    }
    (output.status.success(), stdout, stderr)
}

pub fn run_cmd_quiet(cmd: &str, args: &[&str]) -> (bool, String) {
    let output = Command::new(cmd).args(args).output().unwrap_or_else(|e| {
        panic!("{cmd} 실행 실패: {e}");
    });
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (output.status.success(), stdout)
}

pub fn ssh_cmd(host: &str, user: &str, remote_cmd: &str) -> (bool, String) {
    run_cmd_quiet("ssh", &[
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=5",
        &format!("{user}@{host}"),
        remote_cmd,
    ])
}

pub fn run_self(args: &[&str]) -> String {
    match Command::new("mac-host-commands").args(args).output() {
        Ok(o) => format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
        Err(e) => format!("Error: {}", e),
    }
}

pub fn ensure_dir(path: &Path) {
    if !path.exists() {
        fs::create_dir_all(path).unwrap_or_else(|e| {
            eprintln!("[error] 디렉토리 생성 실패 ({}): {e}", path.display());
            std::process::exit(1);
        });
    }
}
