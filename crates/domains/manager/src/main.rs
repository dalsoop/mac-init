use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const GITHUB_REPO: &str = "dalsoop/mac-app-init";
const DOMAINS_DIR: &str = ".mac-app-init/domains";

#[derive(Parser)]
#[command(name = "mac")]
#[command(about = "macOS 도메인 패키지 매니저")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 설치된 도메인 목록
    List,
    /// 사용 가능한 도메인 (GitHub)
    Available,
    /// 도메인 설치
    Install { name: String },
    /// 도메인 삭제
    Remove { name: String },
    /// 도메인 업데이트
    Update { name: String },
    /// 전체 도메인 업데이트
    UpdateAll,
    /// mac 매니저 자체 업데이트
    SelfUpdate,
    /// 전체 업그레이드 (매니저 + 모든 도메인)
    Upgrade,
    /// 초기 설정 (자동 업데이트 LaunchAgent 등록)
    Setup,
    /// 설정 상태 확인
    Doctor,
    /// 도메인 실행 (mac run keyboard status)
    Run { name: String, args: Vec<String> },
    /// 스케줄 tick (LaunchAgent에서 매분 호출 — 내부용)
    Tick,
    /// 스케줄 작업 목록
    ScheduleList,
    /// 스케줄 작업 추가
    ScheduleAdd {
        name: String,
        command: String,
        #[arg(long)]
        cron: Option<String>,
        #[arg(long)]
        interval: Option<u64>,
    },
    /// 스케줄 작업 삭제
    ScheduleRemove { name: String },
    /// 스케줄 작업 토글
    ScheduleToggle { name: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct Registry {
    installed: Vec<InstalledDomain>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstalledDomain {
    name: String,
    version: String,
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

fn domains_dir() -> PathBuf {
    PathBuf::from(home()).join(DOMAINS_DIR)
}

fn registry_path() -> PathBuf {
    domains_dir().join("registry.json")
}

fn domain_bin_path(name: &str) -> PathBuf {
    domains_dir().join(format!("mac-domain-{}", name))
}

fn load_registry() -> Registry {
    let path = registry_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(Registry { installed: vec![] })
    } else {
        Registry { installed: vec![] }
    }
}

fn save_registry(reg: &Registry) {
    let path = registry_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    let json = serde_json::to_string_pretty(reg).unwrap();
    fs::write(&path, json).expect("registry 저장 실패");
}

fn arch() -> &'static str {
    if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" }
}

fn asset_name(domain: &str) -> String {
    format!("mac-domain-{}-{}-apple-darwin.tar.gz", domain, arch())
}

fn known_domains() -> Vec<&'static str> {
    vec![
        "bootstrap", "keyboard", "connect", "container", "cron", "defaults", "dotfiles", "git", "quickaction", "vscode", "wireguard",
        "files", "projects", "worktree",
        // infra domains (available but not installed by default)
        "mount", "network", "ssh", "proxmox", "synology",
        "setup", "workspace", "github", "obsidian",
        "openclaw", "init",
    ]
}

const LAUNCHAGENT_LABEL: &str = "com.mac-app-init.scheduler";

fn launchagent_path() -> PathBuf {
    PathBuf::from(home()).join(format!("Library/LaunchAgents/{}.plist", LAUNCHAGENT_LABEL))
}

fn is_setup_done() -> bool {
    launchagent_path().exists()
}

fn main() {
    let cli = Cli::parse();

    // First-run check (except for setup/doctor themselves)
    match &cli.command {
        Commands::Setup | Commands::Doctor => {}
        _ => {
            if !is_setup_done() {
                eprintln!("⚠ mac-app-init 초기 설정이 필요합니다.");
                eprintln!("  mac setup  — 자동 업데이트 등록");
                eprintln!();
            }
        }
    }

    match cli.command {
        Commands::List => cmd_list(),
        Commands::Available => cmd_available(),
        Commands::Install { name } => cmd_install(&name),
        Commands::Remove { name } => cmd_remove(&name),
        Commands::Update { name } => cmd_update(&name),
        Commands::UpdateAll => cmd_update_all(),
        Commands::SelfUpdate => cmd_self_update(),
        Commands::Upgrade => cmd_upgrade(),
        Commands::Setup => cmd_setup(),
        Commands::Doctor => cmd_doctor(),
        Commands::Run { name, args } => cmd_run(&name, &args),
        Commands::Tick => cmd_tick(),
        Commands::ScheduleList => cmd_schedule_list(),
        Commands::ScheduleAdd { name, command, cron, interval } => cmd_schedule_add(&name, &command, cron, interval),
        Commands::ScheduleRemove { name } => cmd_schedule_remove(&name),
        Commands::ScheduleToggle { name } => cmd_schedule_toggle(&name),
    }
}

fn cmd_list() {
    let reg = load_registry();
    if reg.installed.is_empty() {
        println!("설치된 도메인이 없습니다.");
        println!("  mac install keyboard  — 도메인 설치");
        return;
    }
    println!("{:<20} {:<10} {}", "DOMAIN", "VERSION", "PATH");
    println!("{}", "─".repeat(60));
    for d in &reg.installed {
        let bin = domain_bin_path(&d.name);
        let exists = if bin.exists() { "✓" } else { "✗ missing" };
        println!("{:<20} {:<10} {}", d.name, d.version, exists);
    }
}

fn cmd_available() {
    let reg = load_registry();
    let installed: Vec<&str> = reg.installed.iter().map(|d| d.name.as_str()).collect();

    println!("{:<20} {}", "DOMAIN", "STATUS");
    println!("{}", "─".repeat(40));
    for name in known_domains() {
        let status = if installed.contains(&name) { "✓ installed" } else { "  available" };
        println!("{:<20} {}", name, status);
    }
}

fn domain_deps(name: &str) -> &'static [&'static str] {
    match name {
        "mount" => &["connect"],
        _ => &[],
    }
}

fn cmd_install(name: &str) {
    let mut reg = load_registry();
    if reg.installed.iter().any(|d| d.name == name) {
        println!("'{}' 이미 설치되어 있습니다. 업데이트: mac update {}", name, name);
        return;
    }

    // 의존성 체크
    let deps = domain_deps(name);
    let missing: Vec<&&str> = deps.iter()
        .filter(|d| !reg.installed.iter().any(|inst| &inst.name == *d))
        .collect();
    if !missing.is_empty() {
        eprintln!("✗ '{}' 은 다음 도메인이 먼저 필요합니다:", name);
        for d in &missing {
            eprintln!("    mac install {}", d);
        }
        return;
    }

    println!("Installing {}...", name);
    match download_domain(name) {
        Ok(version) => {
            reg.installed.push(InstalledDomain {
                name: name.to_string(),
                version,
            });
            save_registry(&reg);
            println!("✓ {} 설치 완료", name);
        }
        Err(e) => eprintln!("✗ 설치 실패: {}", e),
    }
}

fn cmd_remove(name: &str) {
    let mut reg = load_registry();
    let before = reg.installed.len();
    reg.installed.retain(|d| d.name != name);

    if reg.installed.len() == before {
        println!("'{}' 설치되어 있지 않습니다.", name);
        return;
    }

    let bin = domain_bin_path(name);
    if bin.exists() {
        fs::remove_file(&bin).ok();
    }

    save_registry(&reg);
    println!("✓ {} 삭제 완료", name);
}

fn cmd_update(name: &str) {
    let mut reg = load_registry();
    let entry = reg.installed.iter_mut().find(|d| d.name == name);

    match entry {
        Some(d) => {
            println!("Updating {}...", name);
            match download_domain(name) {
                Ok(version) => {
                    d.version = version;
                    save_registry(&reg);
                    println!("✓ {} 업데이트 완료", name);
                }
                Err(e) => eprintln!("✗ 업데이트 실패: {}", e),
            }
        }
        None => println!("'{}' 설치되어 있지 않습니다. 먼저: mac install {}", name, name),
    }
}

fn cmd_update_all() {
    let reg = load_registry();
    let names: Vec<String> = reg.installed.iter().map(|d| d.name.clone()).collect();
    if names.is_empty() {
        println!("설치된 도메인이 없습니다.");
        return;
    }
    for name in &names {
        cmd_update(name);
    }
}

fn cmd_self_update() {
    println!("Updating mac manager...");

    let asset = format!("mac-{}-apple-darwin.tar.gz", arch());
    let dest_dir = std::env::temp_dir();

    // Get latest release tag
    let output = Command::new("gh")
        .args(["release", "list", "--repo", GITHUB_REPO, "--limit", "1", "--exclude-pre-releases", "--json", "tagName"])
        .output();

    let tag = match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let releases: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap_or_default();
            releases.first()
                .and_then(|r| r.get("tagName"))
                .and_then(|v| v.as_str())
                .unwrap_or("latest")
                .to_string()
        }
        _ => {
            eprintln!("✗ 릴리스 확인 실패");
            return;
        }
    };

    // Download
    let result = Command::new("gh")
        .args([
            "release", "download", &tag,
            "--repo", GITHUB_REPO,
            "--pattern", &asset,
            "--dir", &dest_dir.to_string_lossy(),
            "--clobber",
        ])
        .output();

    if result.is_err() || !result.unwrap().status.success() {
        eprintln!("✗ 다운로드 실패");
        return;
    }

    // Extract to temp
    let tar_path = dest_dir.join(&asset);
    let extract = Command::new("tar")
        .args(["xzf", &tar_path.to_string_lossy(), "-C", &dest_dir.to_string_lossy()])
        .output();

    if extract.is_err() || !extract.unwrap().status.success() {
        eprintln!("✗ 압축 해제 실패");
        return;
    }

    // Replace current binary
    let new_bin = dest_dir.join("mac");
    let current_bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/local/bin/mac"));

    if let Err(e) = fs::copy(&new_bin, &current_bin) {
        eprintln!("✗ 바이너리 교체 실패: {}", e);
        eprintln!("  수동: cp {} {}", new_bin.display(), current_bin.display());
        return;
    }

    // Cleanup
    fs::remove_file(&tar_path).ok();
    fs::remove_file(&new_bin).ok();

    println!("✓ mac 매니저 업데이트 완료 ({})", tag);
}

fn cmd_upgrade() {
    println!("=== 전체 업그레이드 ===\n");

    // 1. Self update
    println!("[1] mac 매니저 업데이트");
    cmd_self_update();

    // 2. Update all domains
    println!("\n[2] 설치된 도메인 업데이트");
    cmd_update_all();

    println!("\n=== 업그레이드 완료 ===");
}

fn cmd_run(name: &str, args: &[String]) {
    let bin = domain_bin_path(name);
    if !bin.exists() {
        eprintln!("'{}' 도메인이 설치되어 있지 않습니다.", name);
        eprintln!("  mac install {}", name);
        return;
    }
    let status = Command::new(&bin)
        .args(args)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("실행 실패: {}", e);
            std::process::exit(1);
        });
    std::process::exit(status.code().unwrap_or(1));
}

fn download_domain(name: &str) -> Result<String, String> {
    let asset = asset_name(name);

    // Get latest release
    let output = Command::new("gh")
        .args(["release", "list", "--repo", GITHUB_REPO, "--limit", "1", "--exclude-pre-releases", "--json", "tagName"])
        .output()
        .map_err(|e| format!("gh CLI 필요: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let releases: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).map_err(|e| format!("릴리스 파싱 실패: {}", e))?;

    let tag = releases
        .first()
        .and_then(|r| r.get("tagName"))
        .and_then(|v| v.as_str())
        .ok_or("릴리스를 찾을 수 없습니다")?
        .to_string();

    // Download asset
    let dest_dir = domains_dir();
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let tar_path = dest_dir.join(&asset);
    let result = Command::new("gh")
        .args([
            "release", "download", &tag,
            "--repo", GITHUB_REPO,
            "--pattern", &asset,
            "--dir", &dest_dir.to_string_lossy(),
            "--clobber",
        ])
        .output()
        .map_err(|e| format!("다운로드 실패: {}", e))?;

    if !result.status.success() {
        return Err(format!(
            "다운로드 실패: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }

    // Extract
    let extract = Command::new("tar")
        .args(["xzf", &tar_path.to_string_lossy(), "-C", &dest_dir.to_string_lossy()])
        .output()
        .map_err(|e| format!("압축 해제 실패: {}", e))?;

    if !extract.status.success() {
        return Err("압축 해제 실패".into());
    }

    // Cleanup tar
    fs::remove_file(&tar_path).ok();

    // Make executable
    let bin = domain_bin_path(name);
    Command::new("chmod").args(["+x", &bin.to_string_lossy()]).output().ok();

    Ok(tag)
}

fn cmd_setup() {
    println!("=== mac-app-init 초기 설정 ===\n");

    let plist_path = launchagent_path();

    let mac_bin = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from(format!("{}/.cargo/bin/mac", home())));

    let h = home();
    let log_dir = format!("{}/문서/시스템/로그", h);
    fs::create_dir_all(&log_dir).ok();
    fs::create_dir_all(domains_dir()).ok();

    // LaunchAgent: scheduler tick (매분)
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
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
</plist>"#, label = LAUNCHAGENT_LABEL, bin = mac_bin.display(), log_dir = log_dir);

    if let Err(e) = fs::write(&plist_path, plist) {
        println!("  ✗ LaunchAgent 생성 실패: {}", e);
        return;
    }

    let _ = Command::new("launchctl").args(["unload", &plist_path.to_string_lossy()]).output();
    let load = Command::new("launchctl").args(["load", &plist_path.to_string_lossy()]).output();
    match load {
        Ok(o) if o.status.success() => println!("  ✓ scheduler 등록 완료 (매분 tick)"),
        _ => println!("  ⚠ plist 생성됨, 로드 실패"),
    }

    // Default schedule: daily mac upgrade at 10:00
    let mut sched = load_schedule();
    if !sched.jobs.iter().any(|j| j.name == "mac-upgrade") {
        sched.jobs.push(Job {
            name: "mac-upgrade".into(),
            command: format!("{} upgrade", mac_bin.display()),
            schedule: ScheduleSpec { stype: "cron".into(), cron: Some("0 10 * * *".into()), interval_seconds: None, watch_path: None },
            enabled: true,
            description: "매일 10시 mac + 도메인 자동 업데이트".into(),
        });
        save_schedule(&sched);
        println!("  ✓ mac-upgrade 작업 추가 (매일 10:00)");
    }

    // Create dirs
    println!("  ✓ ~/.mac-app-init/domains/ 생성됨");

    println!("\n=== 완료 ===");
    println!("  mac available     — 사용 가능한 도메인");
    println!("  mac install cron  — 도메인 설치");
    println!("  mac doctor        — 설정 상태 확인");
}

fn cmd_doctor() {
    println!("=== mac-app-init 상태 ===\n");

    // 1. mac binary
    let mac_bin = std::env::current_exe().unwrap_or_default();
    println!("[mac 바이너리] ✓ {}", mac_bin.display());

    // 2. domains dir
    let dd = domains_dir();
    println!("[domains 디렉토리] {}", if dd.exists() { "✓" } else { "✗" });

    // 3. registry
    let reg = load_registry();
    println!("[설치된 도메인] {} 개", reg.installed.len());
    for d in &reg.installed {
        let bin = domain_bin_path(&d.name);
        let ok = bin.exists();
        println!("  {} {} ({})", if ok { "✓" } else { "✗" }, d.name, d.version);
    }

    // 4. LaunchAgent
    let la = launchagent_path();
    if la.exists() {
        println!("[자동 업데이트] ✓ 등록됨 (매일 10:00)");
    } else {
        println!("[자동 업데이트] ✗ 미등록");
        println!("  → mac setup");
    }

    // 5. Dependencies
    println!("\n[의존성]");
    for (name, cmd, args) in &[
        ("gh", "gh", &["--version"] as &[&str]),
        ("dotenvx", "dotenvx", &["--version"]),
        ("nickel", "nickel", &["--version"]),
    ] {
        let ok = Command::new(cmd).args(*args).output().map(|o| o.status.success()).unwrap_or(false);
        println!("  {} {}", if ok { "✓" } else { "✗" }, name);
    }

    // 6. .env
    let env_path = PathBuf::from(home()).join(".env");
    if env_path.exists() {
        let content = fs::read_to_string(&env_path).unwrap_or_default();
        let encrypted = content.contains("encrypted:");
        println!("\n[.env] ✓ 존재 ({})", if encrypted { "암호화됨" } else { "평문" });
    } else {
        println!("\n[.env] ✗ 없음");
        println!("  → mac run bootstrap install");
    }
}

// === Scheduler ===

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    name: String,
    command: String,
    schedule: ScheduleSpec,
    #[serde(default = "true_default")]
    enabled: bool,
    #[serde(default)]
    description: String,
}
fn true_default() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScheduleSpec {
    #[serde(rename = "type")]
    stype: String,
    #[serde(default)]
    cron: Option<String>,
    #[serde(default)]
    interval_seconds: Option<u64>,
    #[serde(default)]
    watch_path: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ScheduleFile {
    jobs: Vec<Job>,
}

fn schedule_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/schedule.json")
}

fn last_run_path() -> PathBuf {
    PathBuf::from(home()).join(".mac-app-init/scheduler-last-run.json")
}

fn schedule_log_path() -> PathBuf {
    PathBuf::from(home()).join("문서/시스템/로그/scheduler.log")
}

fn load_schedule() -> ScheduleFile {
    let path = schedule_path();
    if !path.exists() { return ScheduleFile::default(); }
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_schedule(s: &ScheduleFile) {
    let path = schedule_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    fs::write(&path, serde_json::to_string_pretty(s).unwrap()).ok();
}

fn now_parts() -> (u32, u32, u32, u32, u32) {
    let s = Command::new("date").args(["+%M %H %d %m %u"]).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let p: Vec<u32> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    if p.len() >= 5 { (p[0], p[1], p[2], p[3], p[4] % 7) } else { (0, 0, 0, 0, 0) }
}

fn cron_matches(expr: &str, min: u32, hour: u32, day: u32, month: u32, weekday: u32) -> bool {
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

fn cmd_tick() {
    let sched = load_schedule();
    let (min, hour, day, month, weekday) = now_parts();
    let last_run_path = last_run_path();
    let mut last_run: HashMap<String, u64> = if last_run_path.exists() {
        serde_json::from_str(&fs::read_to_string(&last_run_path).unwrap_or_default()).unwrap_or_default()
    } else { HashMap::new() };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    let log = schedule_log_path();
    fs::create_dir_all(log.parent().unwrap()).ok();

    for job in &sched.jobs {
        if !job.enabled { continue; }
        let should_run = match job.schedule.stype.as_str() {
            "cron" => job.schedule.cron.as_ref().map(|e| cron_matches(e, min, hour, day, month, weekday)).unwrap_or(false),
            "interval" => job.schedule.interval_seconds.map(|s| now - last_run.get(&job.name).copied().unwrap_or(0) >= s).unwrap_or(false),
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
    fs::write(&last_run_path, serde_json::to_string(&last_run).unwrap()).ok();
}

fn cmd_schedule_list() {
    let s = load_schedule();
    if s.jobs.is_empty() {
        println!("등록된 작업이 없습니다.");
        println!("  mac schedule-add <name> <command> --cron \"0 9 * * *\"");
        return;
    }
    println!("{:<20} {:<8} {:<25} {}", "NAME", "STATUS", "SCHEDULE", "COMMAND");
    println!("{}", "─".repeat(80));
    for j in &s.jobs {
        let st = if j.enabled { "✓" } else { "✗" };
        let sc = match j.schedule.stype.as_str() {
            "cron" => j.schedule.cron.clone().unwrap_or_default(),
            "interval" => format!("every {}s", j.schedule.interval_seconds.unwrap_or(0)),
            _ => "?".into(),
        };
        println!("{:<20} {:<8} {:<25} {}", j.name, st, sc, j.command);
    }
}

fn cmd_schedule_add(name: &str, command: &str, cron: Option<String>, interval: Option<u64>) {
    let mut s = load_schedule();
    if s.jobs.iter().any(|j| j.name == name) {
        println!("'{}' 이미 존재합니다.", name);
        return;
    }
    let schedule = if let Some(c) = cron {
        ScheduleSpec { stype: "cron".into(), cron: Some(c), interval_seconds: None, watch_path: None }
    } else if let Some(i) = interval {
        ScheduleSpec { stype: "interval".into(), cron: None, interval_seconds: Some(i), watch_path: None }
    } else {
        eprintln!("--cron 또는 --interval 필요");
        return;
    };
    s.jobs.push(Job {
        name: name.into(), command: command.into(), schedule,
        enabled: true, description: String::new(),
    });
    save_schedule(&s);
    println!("✓ {} 추가", name);
}

fn cmd_schedule_remove(name: &str) {
    let mut s = load_schedule();
    let before = s.jobs.len();
    s.jobs.retain(|j| j.name != name);
    if s.jobs.len() == before {
        println!("'{}' 없음", name);
        return;
    }
    save_schedule(&s);
    println!("✓ {} 삭제", name);
}

fn cmd_schedule_toggle(name: &str) {
    let mut s = load_schedule();
    if let Some(j) = s.jobs.iter_mut().find(|j| j.name == name) {
        j.enabled = !j.enabled;
        let enabled = j.enabled;
        save_schedule(&s);
        println!("{} {}", name, if enabled { "✓ 활성화" } else { "✗ 비활성화" });
    } else {
        println!("'{}' 없음", name);
    }
}
