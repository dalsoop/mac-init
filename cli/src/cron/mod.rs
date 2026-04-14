use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::common;

fn agents_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("Library/LaunchAgents")
}

struct AgentInfo {
    label: String,
    path: PathBuf,
    program: String,
    schedule: String,
    loaded: bool,
    running: bool,
    pid: Option<u32>,
}

fn scan_agents() -> Vec<AgentInfo> {
    let dir = agents_dir();
    if !dir.is_dir() {
        return Vec::new();
    }

    let loaded_services = get_loaded_services();

    let mut agents: Vec<AgentInfo> = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "plist").unwrap_or(false) {
                let label = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                let (program, schedule) = read_plist_info(&path);
                let service = loaded_services.get(&label);
                let loaded = service.is_some();
                let pid = service.and_then(|p| *p);

                agents.push(AgentInfo {
                    label,
                    path,
                    program,
                    schedule,
                    loaded,
                    running: pid.is_some(),
                    pid,
                });
            }
        }
    }

    agents.sort_by(|a, b| a.label.cmp(&b.label));
    agents
}

fn get_loaded_services() -> HashMap<String, Option<u32>> {
    let output = Command::new("launchctl").arg("list").output();
    let mut map = HashMap::new();
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let pid = parts[0].parse::<u32>().ok();
                map.insert(parts[2].to_string(), pid);
            }
        }
    }
    map
}

fn read_plist_info(path: &PathBuf) -> (String, String) {
    let output = Command::new("plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(path)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return (String::new(), String::new()),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return (String::new(), String::new()),
    };

    // Program
    let program = if let Some(prog) = json.get("Program").and_then(|v| v.as_str()) {
        prog.to_string()
    } else if let Some(args) = json.get("ProgramArguments").and_then(|v| v.as_array()) {
        args.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        String::new()
    };

    // Schedule
    let schedule = if json.get("StartCalendarInterval").is_some() {
        let cal = &json["StartCalendarInterval"];
        if cal.is_array() {
            "calendar (multiple)".to_string()
        } else {
            let hour = cal.get("Hour").and_then(|v| v.as_u64());
            let min = cal.get("Minute").and_then(|v| v.as_u64());
            let day = cal.get("Day").and_then(|v| v.as_u64());
            let weekday = cal.get("Weekday").and_then(|v| v.as_u64());
            let mut parts = Vec::new();
            if let Some(d) = day {
                parts.push(format!("day={}", d));
            }
            if let Some(w) = weekday {
                let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                parts.push(names.get(w as usize).unwrap_or(&"?").to_string());
            }
            if let Some(h) = hour {
                if let Some(m) = min {
                    parts.push(format!("{:02}:{:02}", h, m));
                } else {
                    parts.push(format!("{:02}:00", h));
                }
            }
            if parts.is_empty() {
                "calendar".to_string()
            } else {
                parts.join(" ")
            }
        }
    } else if json.get("StartInterval").is_some() {
        let secs = json["StartInterval"].as_u64().unwrap_or(0);
        if secs >= 3600 {
            format!("every {}h", secs / 3600)
        } else if secs >= 60 {
            format!("every {}m", secs / 60)
        } else {
            format!("every {}s", secs)
        }
    } else if json.get("WatchPaths").is_some() {
        let paths = json["WatchPaths"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        format!("watch: {}", paths)
    } else if json
        .get("RunAtLoad")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        "run at load".to_string()
    } else if json.get("KeepAlive").is_some() {
        "keep alive".to_string()
    } else {
        "manual".to_string()
    };

    (program, schedule)
}

pub fn status() {
    println!("=== LaunchAgents (스케줄 작업) ===\n");

    let agents = scan_agents();
    if agents.is_empty() {
        println!("  등록된 LaunchAgent가 없습니다.");
        return;
    }

    // Group: mine vs third-party
    let (mine, others): (Vec<_>, Vec<_>) = agents.iter().partition(|a| {
        a.label.starts_with("com.mac-host")
            || a.label.starts_with("kr.jeonghan")
            || a.label.starts_with("com.mac-init")
    });

    if !mine.is_empty() {
        println!("[내 서비스]");
        for a in &mine {
            print_agent(a);
        }
        println!();
    }

    if !others.is_empty() {
        println!("[서드파티]");
        for a in &others {
            print_agent(a);
        }
    }

    println!("\n총 {} 개 LaunchAgent", agents.len());
}

fn print_agent(a: &AgentInfo) {
    let status_icon = if a.running {
        "✓"
    } else if a.loaded {
        "○"
    } else {
        "✗"
    };
    let status_text = if a.running {
        format!("running (PID {})", a.pid.unwrap_or(0))
    } else if a.loaded {
        "loaded".to_string()
    } else {
        "stopped".to_string()
    };
    println!(
        "  {} {:<50} {} | {}",
        status_icon, a.label, status_text, a.schedule
    );
}

pub fn list() {
    let agents = scan_agents();
    println!(
        "{:<4} {:<50} {:<20} {}",
        "", "LABEL", "STATUS", "SCHEDULE"
    );
    println!("{}", "─".repeat(100));
    for a in &agents {
        let status = if a.running {
            format!("✓ running ({})", a.pid.unwrap_or(0))
        } else if a.loaded {
            "○ loaded".to_string()
        } else {
            "✗ stopped".to_string()
        };
        println!("{:<4} {:<50} {:<20} {}", "", a.label, status, a.schedule);
    }
}

pub fn info(label: &str) {
    let agents = scan_agents();
    let agent = agents.iter().find(|a| a.label == label || a.label.contains(label));

    match agent {
        Some(a) => {
            println!("=== {} ===\n", a.label);
            println!("  Label:    {}", a.label);
            println!("  Path:     {}", a.path.display());
            println!("  Program:  {}", a.program);
            println!("  Schedule: {}", a.schedule);
            println!(
                "  Status:   {}",
                if a.running {
                    format!("running (PID {})", a.pid.unwrap_or(0))
                } else if a.loaded {
                    "loaded (not running)".to_string()
                } else {
                    "stopped".to_string()
                }
            );

            // Show raw plist
            println!("\n[plist 내용]");
            if let Ok(content) = fs::read_to_string(&a.path) {
                println!("{}", content);
            }
        }
        None => {
            println!("✗ '{}' 에 해당하는 LaunchAgent를 찾을 수 없습니다.", label);
            println!("  `mac-host-commands cron list` 로 전체 목록을 확인하세요.");
        }
    }
}

pub fn load(label: &str) {
    let agents = scan_agents();
    let agent = agents.iter().find(|a| a.label == label || a.label.contains(label));

    match agent {
        Some(a) => {
            if a.loaded {
                println!("  ⚠ '{}' 이미 로드됨", a.label);
                return;
            }
            let (ok, _, stderr) =
                common::run_cmd("launchctl", &["load", &a.path.to_string_lossy()]);
            if ok {
                println!("  ✓ '{}' 로드됨", a.label);
            } else {
                println!("  ✗ 로드 실패: {}", stderr.trim());
            }
        }
        None => println!("  ✗ '{}' 를 찾을 수 없습니다.", label),
    }
}

pub fn unload(label: &str) {
    let agents = scan_agents();
    let agent = agents.iter().find(|a| a.label == label || a.label.contains(label));

    match agent {
        Some(a) => {
            if !a.loaded {
                println!("  ⚠ '{}' 이미 정지됨", a.label);
                return;
            }
            let (ok, _, stderr) =
                common::run_cmd("launchctl", &["unload", &a.path.to_string_lossy()]);
            if ok {
                println!("  ✓ '{}' 정지됨", a.label);
            } else {
                println!("  ✗ 정지 실패: {}", stderr.trim());
            }
        }
        None => println!("  ✗ '{}' 를 찾을 수 없습니다.", label),
    }
}

pub fn restart(label: &str) {
    let agents = scan_agents();
    let agent = agents.iter().find(|a| a.label == label || a.label.contains(label));

    match agent {
        Some(a) => {
            let path_str = a.path.to_string_lossy().to_string();
            println!("  {} 재시작 중...", a.label);
            let _ = Command::new("launchctl")
                .args(["unload", &path_str])
                .output();
            let (ok, _, stderr) = common::run_cmd("launchctl", &["load", &path_str]);
            if ok {
                println!("  ✓ '{}' 재시작됨", a.label);
            } else {
                println!("  ✗ 재시작 실패: {}", stderr.trim());
            }
        }
        None => println!("  ✗ '{}' 를 찾을 수 없습니다.", label),
    }
}

pub fn logs(label: &str) {
    println!("=== {} 로그 ===\n", label);

    // Check common log locations
    let home = std::env::var("HOME").unwrap_or_default();
    let log_paths = [
        format!("{}/文档/시스템/로그/{}.log", home, label),
        format!("{}/Library/Logs/{}.log", home, label),
        format!("/tmp/{}.log", label),
    ];

    let mut found = false;
    for path in &log_paths {
        if std::path::Path::new(path).exists() {
            println!("[{}]\n", path);
            // tail -20
            let (ok, stdout, _) = common::run_cmd("tail", &["-20", path]);
            if ok {
                println!("{}", stdout);
            }
            found = true;
            break;
        }
    }

    if !found {
        // Fallback: system log
        println!("[system log (최근 20줄)]");
        let (ok, stdout, _) = common::run_cmd(
            "log",
            &[
                "show",
                "--predicate",
                &format!("subsystem == '{}'", label),
                "--last",
                "5m",
                "--style",
                "compact",
            ],
        );
        if ok && !stdout.trim().is_empty() {
            // Show last 20 lines
            let lines: Vec<&str> = stdout.lines().collect();
            let start = lines.len().saturating_sub(20);
            for line in &lines[start..] {
                println!("{}", line);
            }
        } else {
            println!("  로그를 찾을 수 없습니다.");
            println!("  시도할 경로:");
            for p in &log_paths {
                println!("    {}", p);
            }
        }
    }
}
