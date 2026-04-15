use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::common;
use crate::models::cron::{Job, LaunchAgent, ScheduleFile, ScheduleSpec};

// === Schedule (schedule.json) ===

pub fn schedule_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".mac-app-init/schedule.json")
}

pub fn load_schedule() -> ScheduleFile {
    let path = schedule_path();
    if !path.exists() { return ScheduleFile::default(); }
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_schedule(s: &ScheduleFile) -> Result<(), String> {
    let path = schedule_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, serde_json::to_string_pretty(s).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn add_job(name: &str, command: &str, cron: Option<String>, interval: Option<u64>) -> Result<String, String> {
    let mut s = load_schedule();
    if s.jobs.iter().any(|j| j.name == name) {
        return Err(format!("'{}' 이미 존재합니다", name));
    }
    let schedule = if let Some(c) = cron {
        ScheduleSpec { stype: "cron".into(), cron: Some(c), interval_seconds: None, watch_path: None }
    } else if let Some(i) = interval {
        ScheduleSpec { stype: "interval".into(), cron: None, interval_seconds: Some(i), watch_path: None }
    } else {
        return Err("--cron 또는 --interval 필요".into());
    };
    s.jobs.push(Job {
        name: name.into(), command: command.into(), schedule,
        enabled: true, description: String::new(),
    });
    save_schedule(&s)?;
    Ok(format!("'{}' 추가됨", name))
}

pub fn remove_job(name: &str) -> Result<String, String> {
    let mut s = load_schedule();
    let before = s.jobs.len();
    s.jobs.retain(|j| j.name != name);
    if s.jobs.len() == before {
        return Err(format!("'{}' 이(가) 없습니다", name));
    }
    save_schedule(&s)?;
    Ok(format!("'{}' 삭제됨", name))
}

pub fn toggle_job(name: &str) -> Result<(String, bool), String> {
    let mut s = load_schedule();
    let job = s.jobs.iter_mut().find(|j| j.name == name)
        .ok_or_else(|| format!("'{}' 이(가) 없습니다", name))?;
    job.enabled = !job.enabled;
    let enabled = job.enabled;
    save_schedule(&s)?;
    Ok((name.into(), enabled))
}

// === Tick engine ===

fn last_run_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".mac-app-init/scheduler-last-run.json")
}

fn schedule_log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("문서/시스템/로그/scheduler.log")
}

fn now_parts() -> (u32, u32, u32, u32, u32) {
    let s = Command::new("date").args(["+%M %H %d %m %u"]).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let p: Vec<u32> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    if p.len() >= 5 { (p[0], p[1], p[2], p[3], p[4] % 7) } else { (0, 0, 0, 0, 0) }
}

pub fn cron_matches(expr: &str, min: u32, hour: u32, day: u32, month: u32, weekday: u32) -> bool {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 { return false; }
    fn fm(field: &str, value: u32) -> bool {
        if field == "*" { return true; }
        if let Ok(n) = field.parse::<u32>() { return n == value; }
        if let Some(step) = field.strip_prefix("*/") {
            if let Ok(n) = step.parse::<u32>() { return n > 0 && value % n == 0; }
        }
        if field.contains(',') {
            return field.split(',').any(|f| f.parse::<u32>().ok() == Some(value));
        }
        false
    }
    fm(parts[0], min) && fm(parts[1], hour) && fm(parts[2], day) && fm(parts[3], month) && fm(parts[4], weekday)
}

/// LaunchAgent 에서 매 분 호출: schedule.json 의 job 들 확인 후 실행.
pub fn tick() {
    let sched = load_schedule();
    let (min, hour, day, month, weekday) = now_parts();
    let last_run_path = last_run_path();
    let mut last_run: HashMap<String, u64> = if last_run_path.exists() {
        serde_json::from_str(&fs::read_to_string(&last_run_path).unwrap_or_default()).unwrap_or_default()
    } else { HashMap::new() };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    let log = schedule_log_path();
    if let Some(parent) = log.parent() { fs::create_dir_all(parent).ok(); }

    for job in &sched.jobs {
        if !job.enabled { continue; }
        let should_run = match job.schedule.stype.as_str() {
            "cron" => job.schedule.cron.as_ref()
                .map(|e| cron_matches(e, min, hour, day, month, weekday))
                .unwrap_or(false),
            "interval" => job.schedule.interval_seconds
                .map(|s| now - last_run.get(&job.name).copied().unwrap_or(0) >= s)
                .unwrap_or(false),
            _ => false,
        };
        if should_run {
            let _ = Command::new("bash").args(["-c", &job.command]).output();
            last_run.insert(job.name.clone(), now);
            if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&log) {
                use std::io::Write;
                let ts = Command::new("date").args(["+%Y-%m-%d %H:%M:%S"]).output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
                let _ = writeln!(f, "[{}] RUN {}: {}", ts, job.name, job.command);
            }
        }
    }
    fs::write(&last_run_path, serde_json::to_string(&last_run).unwrap_or_default()).ok();
}

// === LaunchAgent scheduler install/remove ===

pub const SCHEDULER_LABEL: &str = "com.mac-app-init.scheduler";

pub fn scheduler_plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(format!("Library/LaunchAgents/{}.plist", SCHEDULER_LABEL))
}

pub fn scheduler_installed() -> bool {
    scheduler_plist_path().exists()
}

/// mac 바이너리 경로 (`which mac` 결과)
fn mac_bin() -> String {
    Command::new("which").arg("mac").output()
        .ok()
        .and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else { None })
        .unwrap_or_else(|| "mac".into())
}

pub fn install_scheduler() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let plist_path = scheduler_plist_path();
    let log_dir = format!("{}/문서/시스템/로그", home);
    fs::create_dir_all(&log_dir).ok();
    let mac_bin = mac_bin();

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{mac_bin}</string>
        <string>tick</string>
    </array>
    <key>StartInterval</key>
    <integer>60</integer>
    <key>StandardOutPath</key>
    <string>{log_dir}/scheduler.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/scheduler.log</string>
</dict>
</plist>
"#, label=SCHEDULER_LABEL, mac_bin=mac_bin, log_dir=log_dir);

    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&plist_path, plist).map_err(|e| e.to_string())?;

    let (ok, _, stderr) = common::run_cmd("launchctl", &["load", &plist_path.to_string_lossy()]);
    if ok {
        Ok(format!("✓ scheduler 등록 완료 (매분 tick) — {}", plist_path.display()))
    } else {
        Err(format!("launchctl load 실패: {}", stderr.trim()))
    }
}

pub fn remove_scheduler() -> Result<String, String> {
    let plist_path = scheduler_plist_path();
    if !plist_path.exists() {
        return Err("scheduler 가 설치되어 있지 않습니다".into());
    }
    let _ = Command::new("launchctl")
        .args(["unload", &plist_path.to_string_lossy()])
        .output();
    fs::remove_file(&plist_path).map_err(|e| e.to_string())?;
    Ok(format!("✓ scheduler 제거 완료 — {}", plist_path.display()))
}

fn agents_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("Library/LaunchAgents")
}

fn is_mine(label: &str) -> bool {
    let user = std::env::var("USER").unwrap_or_default();
    label.starts_with("com.mac-host")
        || label.starts_with("com.mac-init")
        || label.contains(&user)
}

// === Data functions (no println, return structs) ===

/// Scan all LaunchAgents and return structured data
pub fn get_agents() -> Vec<LaunchAgent> {
    let dir = agents_dir();
    if !dir.is_dir() {
        return Vec::new();
    }

    let loaded_services = get_loaded_services();
    let mut agents = Vec::new();

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

                agents.push(LaunchAgent {
                    is_mine: is_mine(&label),
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

/// Find agent by label (exact or partial match)
pub fn find_agent(label: &str) -> Option<LaunchAgent> {
    get_agents()
        .into_iter()
        .find(|a| a.label == label || a.label.contains(label))
}

/// Read raw plist content
pub fn read_plist(path: &PathBuf) -> Option<String> {
    fs::read_to_string(path).ok()
}

// === Action functions (return Result) ===

pub fn load_agent(label: &str) -> Result<String, String> {
    let agent = find_agent(label).ok_or_else(|| format!("'{}' 를 찾을 수 없습니다", label))?;
    if agent.loaded {
        return Err(format!("'{}' 이미 로드됨", agent.label));
    }
    let (ok, _, stderr) = common::run_cmd("launchctl", &["load", &agent.path.to_string_lossy()]);
    if ok {
        Ok(format!("'{}' 로드됨", agent.label))
    } else {
        Err(format!("로드 실패: {}", stderr.trim()))
    }
}

pub fn unload_agent(label: &str) -> Result<String, String> {
    let agent = find_agent(label).ok_or_else(|| format!("'{}' 를 찾을 수 없습니다", label))?;
    if !agent.loaded {
        return Err(format!("'{}' 이미 정지됨", agent.label));
    }
    let (ok, _, stderr) = common::run_cmd("launchctl", &["unload", &agent.path.to_string_lossy()]);
    if ok {
        Ok(format!("'{}' 정지됨", agent.label))
    } else {
        Err(format!("정지 실패: {}", stderr.trim()))
    }
}

pub fn restart_agent(label: &str) -> Result<String, String> {
    let agent = find_agent(label).ok_or_else(|| format!("'{}' 를 찾을 수 없습니다", label))?;
    let path_str = agent.path.to_string_lossy().to_string();
    let _ = Command::new("launchctl").args(["unload", &path_str]).output();
    let (ok, _, stderr) = common::run_cmd("launchctl", &["load", &path_str]);
    if ok {
        Ok(format!("'{}' 재시작됨", agent.label))
    } else {
        Err(format!("재시작 실패: {}", stderr.trim()))
    }
}

pub fn get_logs(label: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let log_paths = [
        format!("{}/문서/시스템/로그/{}.log", home, label),
        format!("{}/Library/Logs/{}.log", home, label),
        format!("/tmp/{}.log", label),
    ];

    for path in &log_paths {
        if std::path::Path::new(path).exists() {
            let (ok, stdout, _) = common::run_cmd("tail", &["-20", path]);
            if ok {
                return format!("[{}]\n{}", path, stdout);
            }
        }
    }

    let (ok, stdout, _) = common::run_cmd(
        "log",
        &["show", "--predicate", &format!("subsystem == '{}'", label), "--last", "5m", "--style", "compact"],
    );
    if ok && !stdout.trim().is_empty() {
        let lines: Vec<&str> = stdout.lines().collect();
        let start = lines.len().saturating_sub(20);
        return lines[start..].join("\n");
    }

    "로그를 찾을 수 없습니다.".to_string()
}

// === Internal helpers ===

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

    let program = if let Some(prog) = json.get("Program").and_then(|v| v.as_str()) {
        prog.to_string()
    } else if let Some(args) = json.get("ProgramArguments").and_then(|v| v.as_array()) {
        args.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    };

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
            if let Some(d) = day { parts.push(format!("day={}", d)); }
            if let Some(w) = weekday {
                let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                parts.push(names.get(w as usize).unwrap_or(&"?").to_string());
            }
            if let Some(h) = hour {
                parts.push(if let Some(m) = min { format!("{:02}:{:02}", h, m) } else { format!("{:02}:00", h) });
            }
            if parts.is_empty() { "calendar".to_string() } else { parts.join(" ") }
        }
    } else if json.get("StartInterval").is_some() {
        let secs = json["StartInterval"].as_u64().unwrap_or(0);
        if secs >= 3600 { format!("every {}h", secs / 3600) }
        else if secs >= 60 { format!("every {}m", secs / 60) }
        else { format!("every {}s", secs) }
    } else if json.get("WatchPaths").is_some() {
        let paths = json["WatchPaths"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        format!("watch: {}", paths)
    } else if json.get("RunAtLoad").and_then(|v| v.as_bool()).unwrap_or(false) {
        "run at load".to_string()
    } else if json.get("KeepAlive").is_some() {
        "keep alive".to_string()
    } else {
        "manual".to_string()
    };

    (program, schedule)
}

// === CLI print wrappers (backward compat) ===

pub fn status() {
    let agents = get_agents();
    println!("=== LaunchAgents (스케줄 작업) ===\n");
    if agents.is_empty() { println!("  등록된 LaunchAgent가 없습니다."); return; }

    let (mine, others): (Vec<_>, Vec<_>) = agents.iter().partition(|a| a.is_mine);
    if !mine.is_empty() {
        println!("[내 서비스]");
        for a in &mine { print_agent_line(a); }
        println!();
    }
    if !others.is_empty() {
        println!("[서드파티]");
        for a in &others { print_agent_line(a); }
    }
    println!("\n총 {} 개 LaunchAgent", agents.len());
}

pub fn list() {
    let agents = get_agents();
    println!("{:<4} {:<50} {:<20} {}", "", "LABEL", "STATUS", "SCHEDULE");
    println!("{}", "─".repeat(100));
    for a in &agents {
        let s = if a.running { format!("✓ running ({})", a.pid.unwrap_or(0)) }
                else if a.loaded { "○ loaded".to_string() }
                else { "✗ stopped".to_string() };
        println!("{:<4} {:<50} {:<20} {}", "", a.label, s, a.schedule);
    }
}

pub fn info(label: &str) {
    match find_agent(label) {
        Some(a) => {
            println!("=== {} ===\n", a.label);
            println!("  Label:    {}", a.label);
            println!("  Path:     {}", a.path.display());
            println!("  Program:  {}", a.program);
            println!("  Schedule: {}", a.schedule);
            let s = if a.running { format!("running (PID {})", a.pid.unwrap_or(0)) }
                    else if a.loaded { "loaded (not running)".to_string() }
                    else { "stopped".to_string() };
            println!("  Status:   {}", s);
            if let Some(content) = read_plist(&a.path) {
                println!("\n[plist 내용]\n{}", content);
            }
        }
        None => println!("✗ '{}' 에 해당하는 LaunchAgent를 찾을 수 없습니다.", label),
    }
}

pub fn load(label: &str) {
    match load_agent(label) {
        Ok(msg) => println!("  ✓ {}", msg),
        Err(msg) => println!("  ✗ {}", msg),
    }
}

pub fn unload(label: &str) {
    match unload_agent(label) {
        Ok(msg) => println!("  ✓ {}", msg),
        Err(msg) => println!("  ✗ {}", msg),
    }
}

pub fn restart(label: &str) {
    match restart_agent(label) {
        Ok(msg) => println!("  ✓ {}", msg),
        Err(msg) => println!("  ✗ {}", msg),
    }
}

pub fn logs(label: &str) {
    println!("=== {} 로그 ===\n", label);
    println!("{}", get_logs(label));
}

fn print_agent_line(a: &LaunchAgent) {
    let icon = if a.running { "✓" } else if a.loaded { "○" } else { "✗" };
    let text = if a.running { format!("running (PID {})", a.pid.unwrap_or(0)) }
               else if a.loaded { "loaded".to_string() }
               else { "stopped".to_string() };
    println!("  {} {:<50} {} | {}", icon, a.label, text, a.schedule);
}
