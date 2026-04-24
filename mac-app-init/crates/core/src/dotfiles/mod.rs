use std::fs;
use std::path::PathBuf;

use crate::models::config_entry::{ConfigCategory, ConfigEntry};

struct ConfigSource {
    name: &'static str,
    path: &'static str,
    category: ConfigCategory,
}

const KNOWN_CONFIGS: &[ConfigSource] = &[
    ConfigSource {
        name: "zshrc",
        path: "~/.zshrc",
        category: ConfigCategory::Shell,
    },
    ConfigSource {
        name: "zprofile",
        path: "~/.zprofile",
        category: ConfigCategory::Shell,
    },
    ConfigSource {
        name: "bash_profile",
        path: "~/.bash_profile",
        category: ConfigCategory::Shell,
    },
    ConfigSource {
        name: "gitconfig",
        path: "~/.gitconfig",
        category: ConfigCategory::Git,
    },
    ConfigSource {
        name: "git/config",
        path: "~/.config/git/config",
        category: ConfigCategory::Git,
    },
    ConfigSource {
        name: "ssh/config",
        path: "~/.ssh/config",
        category: ConfigCategory::Ssh,
    },
    ConfigSource {
        name: "karabiner",
        path: "~/.config/karabiner/karabiner.json",
        category: ConfigCategory::Keyboard,
    },
    ConfigSource {
        name: "tmux.conf",
        path: "~/.tmux.conf",
        category: ConfigCategory::Terminal,
    },
    ConfigSource {
        name: "tmux (xdg)",
        path: "~/.config/tmux/tmux.conf",
        category: ConfigCategory::Terminal,
    },
    ConfigSource {
        name: "iterm2",
        path: "~/.config/iterm2",
        category: ConfigCategory::Terminal,
    },
    ConfigSource {
        name: "vscode settings",
        path: "~/Library/Application Support/Code/User/settings.json",
        category: ConfigCategory::Editor,
    },
    ConfigSource {
        name: "vscode keybindings",
        path: "~/Library/Application Support/Code/User/keybindings.json",
        category: ConfigCategory::Editor,
    },
];

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(path)
}

pub fn scan_configs() -> Vec<ConfigEntry> {
    let mut entries = Vec::new();

    for src in KNOWN_CONFIGS {
        let path = expand_tilde(src.path);
        if path.exists() {
            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
                    Some(format_date(secs))
                })
                .unwrap_or_else(|| "unknown".to_string());

            entries.push(ConfigEntry {
                name: src.name.to_string(),
                path: path.clone(),
                category: src.category.clone(),
                size_bytes: if metadata.is_file() {
                    metadata.len()
                } else {
                    dir_size(&path)
                },
                modified,
            });
        }
    }

    // Scan ~/.config/ for extra dirs
    let config_dir = expand_tilde("~/.config");
    if config_dir.is_dir() {
        if let Ok(read_dir) = fs::read_dir(&config_dir) {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entries.iter().any(|e| e.name == name) {
                    continue;
                }
                let path = entry.path();
                let metadata = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                entries.push(ConfigEntry {
                    name,
                    path: path.clone(),
                    category: ConfigCategory::Other,
                    size_bytes: if metadata.is_file() {
                        metadata.len()
                    } else {
                        dir_size(&path)
                    },
                    modified: metadata
                        .modified()
                        .ok()
                        .and_then(|t| {
                            let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
                            Some(format_date(secs))
                        })
                        .unwrap_or_else(|| "unknown".to_string()),
                });
            }
        }
    }

    entries.sort_by(|a, b| {
        a.category
            .to_string()
            .cmp(&b.category.to_string())
            .then(a.name.cmp(&b.name))
    });
    entries
}

fn dir_size(path: &PathBuf) -> u64 {
    let mut total = 0u64;
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let m = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if m.is_file() {
                    total += m.len();
                } else if m.is_dir() {
                    total += dir_size(&entry.path());
                }
            }
        }
    }
    total
}

fn format_date(epoch_secs: u64) -> String {
    let days = epoch_secs / 86400;
    let mut y = 1970i64;
    let mut remaining = days as i64;

    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }

    let months_days: &[i64] = if is_leap(y) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0usize;
    for (i, &d) in months_days.iter().enumerate() {
        if remaining < d {
            m = i;
            break;
        }
        remaining -= d;
    }

    format!("{:04}-{:02}-{:02}", y, m + 1, remaining + 1)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn read_config(path: &PathBuf) -> Option<String> {
    fs::read_to_string(path).ok()
}
