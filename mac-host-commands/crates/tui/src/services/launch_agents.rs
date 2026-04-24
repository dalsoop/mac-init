use color_eyre::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::models::LaunchAgent;

pub fn scan_agents() -> Result<Vec<LaunchAgent>> {
    let home = std::env::var("HOME")?;
    let agents_dir = PathBuf::from(&home).join("Library/LaunchAgents");

    if !agents_dir.is_dir() {
        return Ok(Vec::new());
    }

    let loaded = get_loaded_services()?;

    // Collect all plist paths first
    let plist_paths: Vec<PathBuf> = fs::read_dir(&agents_dir)?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "plist"))
        .map(|e| e.path())
        .collect();

    // Batch convert all plists to JSON in one pass
    let programs = batch_read_programs(&plist_paths);

    let mut agents: Vec<LaunchAgent> = plist_paths
        .into_iter()
        .map(|path| {
            let label = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let program = programs.get(&label).cloned().unwrap_or_default();
            let service_info = loaded.get(&label);
            let is_loaded = service_info.is_some();
            let pid = service_info.and_then(|p| *p);

            LaunchAgent {
                label,
                path,
                program,
                loaded: is_loaded,
                running: pid.is_some(),
                pid,
            }
        })
        .collect();

    agents.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(agents)
}

fn get_loaded_services() -> Result<HashMap<String, Option<u32>>> {
    let output = Command::new("launchctl").arg("list").output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = HashMap::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let pid = parts[0].parse::<u32>().ok();
            let label = parts[2].to_string();
            services.insert(label, pid);
        }
    }

    Ok(services)
}

fn batch_read_programs(paths: &[PathBuf]) -> HashMap<String, String> {
    let mut results = HashMap::new();

    for path in paths {
        let label = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let program = read_plist_program(path).unwrap_or_default();
        results.insert(label, program);
    }

    results
}

fn read_plist_program(path: &PathBuf) -> Result<String> {
    let output = Command::new("plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Ok(String::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(prog) = json.get("Program").and_then(|v| v.as_str()) {
            return Ok(prog.to_string());
        }
        if let Some(args) = json.get("ProgramArguments").and_then(|v| v.as_array()) {
            let cmd: Vec<&str> = args.iter().filter_map(|v| v.as_str()).collect();
            return Ok(cmd.join(" "));
        }
    }

    Ok(String::new())
}

pub fn load_agent(label: &str, path: &PathBuf) -> Result<String> {
    let output = Command::new("launchctl")
        .args(["load", &path.to_string_lossy()])
        .output()?;
    let msg = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        Ok(format!("Loaded {}\n{}", label, msg))
    } else {
        Ok(format!("Failed to load {}: {}", label, msg))
    }
}

pub fn unload_agent(label: &str, path: &PathBuf) -> Result<String> {
    let output = Command::new("launchctl")
        .args(["unload", &path.to_string_lossy()])
        .output()?;
    let msg = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        Ok(format!("Unloaded {}\n{}", label, msg))
    } else {
        Ok(format!("Failed to unload {}: {}", label, msg))
    }
}
