use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-scheduler")]
#[command(about = "mac-app-init 통합 스케줄러")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 스케줄 틱 실행 (LaunchAgent에서 매분 호출)
    Tick,
    /// 등록된 작업 목록
    List,
    /// 작업 추가
    Add {
        name: String,
        command: String,
        /// cron 표현식 (예: "0 9 * * *")
        #[arg(long)]
        cron: Option<String>,
        /// 반복 간격 (초)
        #[arg(long)]
        interval: Option<u64>,
        /// 감시 경로
        #[arg(long)]
        watch: Option<String>,
        /// 설명
        #[arg(long, default_value = "")]
        description: String,
    },
    /// 작업 삭제
    Remove { name: String },
    /// 작업 활성화
    Enable { name: String },
    /// 작업 비활성화
    Disable { name: String },
    /// LaunchAgent 설치 (scheduler plist 등록)
    Setup,
    /// 즉시 실행
    Run { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    name: String,
    command: String,
    schedule: Schedule,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    description: String,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Schedule {
    #[serde(rename = "type")]
    stype: String,
    #[serde(default)]
    cron: Option<String>,
    #[serde(default)]
    interval_seconds: Option<u64>,
    #[serde(default)]
    watch_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScheduleFile {
    jobs: Vec<Job>,
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn schedule_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/schedule.json")
}

fn log_path() -> PathBuf {
    PathBuf::from(home()).join("문서/시스템/로그/scheduler.log")
}

fn last_run_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/scheduler-last-run.json")
}

fn load_schedule() -> ScheduleFile {
    let path = schedule_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(ScheduleFile { jobs: vec![] })
    } else {
        // Try loading from NCL
        load_from_ncl().unwrap_or(ScheduleFile { jobs: vec![] })
    }
}

fn load_from_ncl() -> Option<ScheduleFile> {
    // Find schedule.ncl in known locations
    let candidates = [
        PathBuf::from(home()).join("문서/프로젝트/mac-app-init/ncl/schedule.ncl"),
        PathBuf::from(home()).join("Documents/Claude/mac-app-init/ncl/schedule.ncl"),
    ];
    for path in &candidates {
        if path.exists() {
            let output = Command::new("nickel")
                .args(["export", &path.to_string_lossy()])
                .output()
                .ok()?;
            if output.status.success() {
                let json = String::from_utf8_lossy(&output.stdout);
                return serde_json::from_str(&json).ok();
            }
        }
    }
    None
}

fn save_schedule(sched: &ScheduleFile) {
    let path = schedule_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    let json = serde_json::to_string_pretty(sched).unwrap();
    fs::write(&path, json).expect("schedule.json 저장 실패");
}

fn load_last_run() -> std::collections::HashMap<String, u64> {
    let path = last_run_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    }
}

fn save_last_run(map: &std::collections::HashMap<String, u64>) {
    let path = last_run_path();
    let json = serde_json::to_string(map).unwrap();
    fs::write(&path, json).ok();
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn now_parts() -> (u32, u32, u32, u32, u32) {
    // (minute, hour, day, month, weekday)
    let output = Command::new("date").args(["+%M %H %d %m %u"]).output();
    let s = output.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let parts: Vec<u32> = s.split_whitespace().filter_map(|p| p.parse().ok()).collect();
    if parts.len() >= 5 {
        (parts[0], parts[1], parts[2], parts[3], parts[4] % 7) // %u: 1=Mon, convert 7→0 for Sun
    } else {
        (0, 0, 0, 0, 0)
    }
}

fn cron_matches(expr: &str, minute: u32, hour: u32, day: u32, month: u32, weekday: u32) -> bool {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 { return false; }

    fn field_matches(field: &str, value: u32) -> bool {
        if field == "*" { return true; }
        if let Ok(n) = field.parse::<u32>() { return n == value; }
        // */N step
        if let Some(step) = field.strip_prefix("*/") {
            if let Ok(n) = step.parse::<u32>() {
                return n > 0 && value % n == 0;
            }
        }
        // comma-separated
        if field.contains(',') {
            return field.split(',').any(|f| f.parse::<u32>().ok() == Some(value));
        }
        false
    }

    field_matches(parts[0], minute)
        && field_matches(parts[1], hour)
        && field_matches(parts[2], day)
        && field_matches(parts[3], month)
        && field_matches(parts[4], weekday)
}

fn append_log(msg: &str) {
    let path = log_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let ts = Command::new("date").args(["+%Y-%m-%d %H:%M:%S"]).output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let _ = writeln!(f, "[{}] {}", ts, msg);
    }
}

fn run_job(job: &Job) {
    append_log(&format!("RUN: {} → {}", job.name, job.command));
    let output = Command::new("bash").args(["-c", &job.command]).output();
    match output {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            let err = String::from_utf8_lossy(&o.stderr);
            if !out.trim().is_empty() { append_log(&format!("  stdout: {}", out.trim())); }
            if !err.trim().is_empty() { append_log(&format!("  stderr: {}", err.trim())); }
            if o.status.success() {
                append_log(&format!("  OK: {}", job.name));
            } else {
                append_log(&format!("  FAIL: {} (exit {})", job.name, o.status.code().unwrap_or(-1)));
            }
        }
        Err(e) => append_log(&format!("  ERROR: {}", e)),
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Tick => cmd_tick(),
        Commands::List => cmd_list(),
        Commands::Add { name, command, cron, interval, watch, description } =>
            cmd_add(&name, &command, cron, interval, watch, &description),
        Commands::Remove { name } => cmd_remove(&name),
        Commands::Enable { name } => cmd_toggle(&name, true),
        Commands::Disable { name } => cmd_toggle(&name, false),
        Commands::Setup => cmd_setup(),
        Commands::Run { name } => cmd_run_job(&name),
    }
}

fn cmd_tick() {
    let sched = load_schedule();
    let (min, hour, day, month, weekday) = now_parts();
    let mut last_run = load_last_run();
    let now = now_epoch();

    for job in &sched.jobs {
        if !job.enabled { continue; }

        let should_run = match job.schedule.stype.as_str() {
            "cron" => {
                if let Some(expr) = &job.schedule.cron {
                    cron_matches(expr, min, hour, day, month, weekday)
                } else { false }
            }
            "interval" => {
                if let Some(secs) = job.schedule.interval_seconds {
                    let last = last_run.get(&job.name).copied().unwrap_or(0);
                    now - last >= secs
                } else { false }
            }
            "watch" => false, // watch는 별도 처리 (fswatch 등)
            _ => false,
        };

        if should_run {
            run_job(job);
            last_run.insert(job.name.clone(), now);
        }
    }

    save_last_run(&last_run);
}

fn cmd_list() {
    let sched = load_schedule();
    if sched.jobs.is_empty() {
        println!("등록된 작업이 없습니다.");
        println!("  mac run scheduler add my-task \"echo hello\" --cron \"0 9 * * *\"");
        return;
    }

    println!("{:<20} {:<8} {:<20} {}", "NAME", "STATUS", "SCHEDULE", "COMMAND");
    println!("{}", "─".repeat(75));
    for job in &sched.jobs {
        let status = if job.enabled { "✓" } else { "✗" };
        let sched_str = match job.schedule.stype.as_str() {
            "cron" => job.schedule.cron.clone().unwrap_or_default(),
            "interval" => format!("every {}s", job.schedule.interval_seconds.unwrap_or(0)),
            "watch" => format!("watch:{}", job.schedule.watch_path.clone().unwrap_or_default()),
            _ => "?".into(),
        };
        println!("{:<20} {:<8} {:<20} {}", job.name, status, sched_str, job.command);
    }
}

fn cmd_add(name: &str, command: &str, cron: Option<String>, interval: Option<u64>, watch: Option<String>, description: &str) {
    let mut sched = load_schedule();

    if sched.jobs.iter().any(|j| j.name == name) {
        println!("'{}' 이미 존재합니다.", name);
        return;
    }

    let (stype, schedule) = if let Some(c) = cron {
        ("cron", Schedule { stype: "cron".into(), cron: Some(c), interval_seconds: None, watch_path: None })
    } else if let Some(i) = interval {
        ("interval", Schedule { stype: "interval".into(), cron: None, interval_seconds: Some(i), watch_path: None })
    } else if let Some(w) = watch {
        ("watch", Schedule { stype: "watch".into(), cron: None, interval_seconds: None, watch_path: Some(w) })
    } else {
        eprintln!("--cron, --interval, 또는 --watch 중 하나를 지정하세요.");
        return;
    };

    sched.jobs.push(Job {
        name: name.into(),
        command: command.into(),
        schedule,
        enabled: true,
        description: description.into(),
    });

    save_schedule(&sched);
    println!("✓ {} 추가 완료 ({})", name, stype);
}

fn cmd_remove(name: &str) {
    let mut sched = load_schedule();
    let before = sched.jobs.len();
    sched.jobs.retain(|j| j.name != name);
    if sched.jobs.len() == before {
        println!("'{}' 를 찾을 수 없습니다.", name);
        return;
    }
    save_schedule(&sched);
    println!("✓ {} 삭제 완료", name);
}

fn cmd_toggle(name: &str, enabled: bool) {
    let mut sched = load_schedule();
    if let Some(job) = sched.jobs.iter_mut().find(|j| j.name == name) {
        job.enabled = enabled;
        save_schedule(&sched);
        println!("✓ {} {}", name, if enabled { "활성화" } else { "비활성화" });
    } else {
        println!("'{}' 를 찾을 수 없습니다.", name);
    }
}

fn cmd_run_job(name: &str) {
    let sched = load_schedule();
    if let Some(job) = sched.jobs.iter().find(|j| j.name == name) {
        println!("Running {}...", name);
        run_job(job);
        println!("✓ 완료");
    } else {
        println!("'{}' 를 찾을 수 없습니다.", name);
    }
}

fn cmd_setup() {
    println!("=== 스케줄러 설정 ===\n");

    let h = home();
    let mac_domain_bin = format!("{}/.mac-app-init/domains/mac-domain-scheduler", h);

    // Check if binary exists
    let bin = if std::path::Path::new(&mac_domain_bin).exists() {
        mac_domain_bin
    } else {
        // Fallback to current exe
        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("mac-domain-scheduler"))
            .to_string_lossy()
            .to_string()
    };

    let plist_path = format!("{}/Library/LaunchAgents/com.mac-app-init.scheduler.plist", h);
    let log_dir = format!("{}/문서/시스템/로그", h);
    fs::create_dir_all(&log_dir).ok();

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mac-app-init.scheduler</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>tick</string>
    </array>
    <key>StartInterval</key>
    <integer>60</integer>
    <key>StandardOutPath</key>
    <string>{log_dir}/scheduler.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/scheduler.log</string>
</dict>
</plist>"#);

    fs::write(&plist_path, plist).expect("plist 생성 실패");

    let _ = Command::new("launchctl").args(["unload", &plist_path]).output();
    let load = Command::new("launchctl").args(["load", &plist_path]).output();
    match load {
        Ok(o) if o.status.success() => println!("  ✓ 스케줄러 등록 완료 (매분 tick)"),
        _ => println!("  ⚠ plist 생성됨, 로드 실패"),
    }

    // Initialize schedule.json from NCL if not exists
    let json_path = schedule_path();
    if !json_path.exists() {
        if let Some(sched) = load_from_ncl() {
            save_schedule(&sched);
            println!("  ✓ schedule.json 초기화 ({} 작업)", sched.jobs.len());
        }
    } else {
        let sched = load_schedule();
        println!("  ✓ schedule.json 존재 ({} 작업)", sched.jobs.len());
    }

    println!("\n=== 완료 ===");
    println!("  mac run scheduler list  — 작업 목록");
    println!("  mac run scheduler add   — 작업 추가");
}
