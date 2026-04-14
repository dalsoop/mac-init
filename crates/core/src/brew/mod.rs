use std::process::Command;
use crate::models::brew::BrewPackage;

pub fn list_installed() -> Vec<BrewPackage> {
    let mut packages = Vec::new();

    // Formulae
    if let Ok(output) = Command::new("brew").args(["list", "--formula", "--versions"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                packages.push(BrewPackage {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    is_cask: false,
                    outdated: false,
                });
            }
        }
    }

    // Casks
    if let Ok(output) = Command::new("brew").args(["list", "--cask", "--versions"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                packages.push(BrewPackage {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    is_cask: true,
                    outdated: false,
                });
            }
        }
    }

    // Mark outdated
    if let Ok(output) = Command::new("brew").args(["outdated", "--json"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                for section in &["formulae", "casks"] {
                    if let Some(arr) = json.get(section).and_then(|v| v.as_array()) {
                        for item in arr {
                            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                                if let Some(pkg) = packages.iter_mut().find(|p| p.name == name) {
                                    pkg.outdated = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

pub fn install(name: &str, is_cask: bool) -> Result<String, String> {
    let mut args = vec!["install"];
    if is_cask { args.push("--cask"); }
    args.push(name);
    run_brew(&args)
}

pub fn uninstall(name: &str, is_cask: bool) -> Result<String, String> {
    let mut args = vec!["uninstall"];
    if is_cask { args.push("--cask"); }
    args.push(name);
    run_brew(&args)
}

pub fn upgrade(name: &str, is_cask: bool) -> Result<String, String> {
    let mut args = vec!["upgrade"];
    if is_cask { args.push("--cask"); }
    args.push(name);
    run_brew(&args)
}

fn run_brew(args: &[&str]) -> Result<String, String> {
    Command::new("brew")
        .args(args)
        .output()
        .map(|o| {
            format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
        })
        .map_err(|e| format!("brew 실행 실패: {}", e))
}
