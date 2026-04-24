use color_eyre::Result;
use std::process::Command;

use crate::models::BrewPackage;

pub fn list_installed() -> Result<Vec<BrewPackage>> {
    let mut packages = Vec::new();

    // Formulae
    let output = Command::new("brew")
        .args(["list", "--formula", "--versions"])
        .output()?;
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

    // Casks
    let output = Command::new("brew")
        .args(["list", "--cask", "--versions"])
        .output()?;
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

    // Mark outdated
    let output = Command::new("brew")
        .args(["outdated", "--json"])
        .output()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(formulae) = json.get("formulae").and_then(|v| v.as_array()) {
                for f in formulae {
                    if let Some(name) = f.get("name").and_then(|v| v.as_str()) {
                        if let Some(pkg) = packages.iter_mut().find(|p| p.name == name) {
                            pkg.outdated = true;
                        }
                    }
                }
            }
            if let Some(casks) = json.get("casks").and_then(|v| v.as_array()) {
                for c in casks {
                    if let Some(name) = c.get("name").and_then(|v| v.as_str()) {
                        if let Some(pkg) = packages.iter_mut().find(|p| p.name == name) {
                            pkg.outdated = true;
                        }
                    }
                }
            }
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packages)
}

pub fn install(name: &str, is_cask: bool) -> Result<String> {
    let mut args = vec!["install"];
    if is_cask {
        args.push("--cask");
    }
    args.push(name);
    let output = Command::new("brew").args(&args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr))
}

pub fn uninstall(name: &str, is_cask: bool) -> Result<String> {
    let mut args = vec!["uninstall"];
    if is_cask {
        args.push("--cask");
    }
    args.push(name);
    let output = Command::new("brew").args(&args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr))
}

pub fn upgrade(name: &str, is_cask: bool) -> Result<String> {
    let mut args = vec!["upgrade"];
    if is_cask {
        args.push("--cask");
    }
    args.push(name);
    let output = Command::new("brew").args(&args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr))
}
