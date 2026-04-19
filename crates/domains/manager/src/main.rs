use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
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
        "bootstrap", "keyboard", "container", "cron", "git", "quickaction", "vscode", "wireguard",
        "files", "projects", "worktree",
        // infra domains (available but not installed by default)
        "mount", "network", "ssh", "proxmox", "synology",
        "setup", "workspace", "github", "obsidian",
        "env", "host", "shell",  "openclaw", "init",
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
        "mount" => &["env"],
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

    fs::create_dir_all(domains_dir()).ok();
    println!("  ✓ ~/.mac-app-init/domains/ 생성됨");

    // Scheduler LaunchAgent — core 로 위임
    match mac_host_core::cron::install_scheduler() {
        Ok(m) => println!("  {}", m),
        Err(e) => println!("  ⚠ scheduler 설치 실패: {}", e),
    }

    // Default schedule: daily mac upgrade at 10:00
    let mut sched = mac_host_core::cron::load_schedule();
    if !sched.jobs.iter().any(|j| j.name == "mac-upgrade") {
        let mac_bin = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from(format!("{}/.cargo/bin/mac", home())));
        sched.jobs.push(mac_host_core::models::cron::Job {
            name: "mac-upgrade".into(),
            command: format!("{} upgrade", mac_bin.display()),
            schedule: mac_host_core::models::cron::ScheduleSpec {
                stype: "cron".into(),
                cron: Some("0 10 * * *".into()),
                interval_seconds: None,
                watch_path: None,
            },
            enabled: true,
            description: "매일 10시 mac + 도메인 자동 업데이트".into(),
        });
        if let Err(e) = mac_host_core::cron::save_schedule(&sched) {
            println!("  ⚠ schedule 저장 실패: {}", e);
        } else {
            println!("  ✓ mac-upgrade 작업 추가 (매일 10:00)");
        }
    }

    println!("\n=== 완료 ===");
    println!("  mac available     — 사용 가능한 도메인");
    println!("  mac install cron  — cron 도메인 설치 (스케줄 관리 UI)");
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

// === Scheduler (core 위임 + deprecated 래퍼) ===

fn cmd_tick() {
    mac_host_core::cron::tick();
}

fn deprecated_notice(old: &str, new: &str) {
    eprintln!("⚠ `mac {}` 는 deprecated 입니다. `mac run cron {}` 를 사용하세요.", old, new);
}

fn cmd_schedule_list() {
    deprecated_notice("schedule-list", "jobs");
    let s = mac_host_core::cron::load_schedule();
    if s.jobs.is_empty() {
        println!("등록된 작업이 없습니다.");
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
    deprecated_notice("schedule-add", "add <name> <command> --cron \"...\"");
    match mac_host_core::cron::add_job(name, command, cron, interval) {
        Ok(m) => println!("✓ {}", m),
        Err(e) => eprintln!("✗ {}", e),
    }
}

fn cmd_schedule_remove(name: &str) {
    deprecated_notice("schedule-remove", "remove <name>");
    match mac_host_core::cron::remove_job(name) {
        Ok(m) => println!("✓ {}", m),
        Err(e) => eprintln!("✗ {}", e),
    }
}

fn cmd_schedule_toggle(name: &str) {
    deprecated_notice("schedule-toggle", "toggle <name>");
    match mac_host_core::cron::toggle_job(name) {
        Ok((n, en)) => println!("'{}' {}", n, if en { "✓ 활성화" } else { "✗ 비활성화" }),
        Err(e) => eprintln!("✗ {}", e),
    }
}
