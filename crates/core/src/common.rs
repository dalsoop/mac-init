use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/jeonghan".to_string());
    PathBuf::from(home).join(".mac-host-commands")
}

pub fn env_file() -> PathBuf {
    config_dir().join(".env")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load_env() {
    let path = env_file();
    if !path.exists() {
        return;
    }

    let content = fs::read_to_string(&path).unwrap_or_default();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            if std::env::var(key).is_err() && !value.is_empty() {
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

/// Run mac-host-commands CLI and return output (for TUI fallback)
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
