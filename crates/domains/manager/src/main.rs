use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const GITHUB_REPO: &str = "dalsoop/mac-app-init";
const DOMAINS_DIR: &str = ".mac-app-init/domains";

#[derive(Parser)]
#[command(name = "mai")]
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
    /// mai 매니저 자체 업데이트
    SelfUpdate,
    /// 전체 업그레이드 (매니저 + 모든 도메인)
    Upgrade,
    /// 초기 설정 (자동 업데이트 LaunchAgent 등록)
    Setup,
    /// 설정 상태 확인
    Doctor,
    /// 도메인 실행 (mai run keyboard status)
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

/// 도메인 목록. locale.json (ncl SSOT) 에서 읽음. 없으면 fallback.
fn known_domains() -> Vec<String> {
    let presets = mac_common::locale::get_all_domain_names();
    if !presets.is_empty() { return presets; }
    // locale.json 없을 때 fallback
    vec![
        "bootstrap", "env", "mount", "host",
        "cron", "files", "sd-backup",
        "git", "vscode", "container",
        "quickaction", "keyboard", "shell", "wireguard", "tmux",
    ].into_iter().map(String::from).collect()
}

const LAUNCHAGENT_LABEL: &str = "com.mac-app-init.scheduler";

fn launchagent_path() -> PathBuf {
    PathBuf::from(home()).join(format!("Library/LaunchAgents/{}.plist", LAUNCHAGENT_LABEL))
}

fn is_setup_done() -> bool {
    launchagent_path().exists()
}

fn main() {
    // 인자 없이 `mai` 만 실행 → TUI
    if std::env::args().len() <= 1 {
        let candidates = [
            domains_dir().join("mai-tui"),
            PathBuf::from(home()).join(".local/bin/mai-tui"),
        ];
        for tui in &candidates {
            if tui.exists() {
                let err = std::os::unix::process::CommandExt::exec(&mut Command::new(tui));
                eprintln!("TUI 실행 실패: {}", err);
                std::process::exit(1);
            }
        }
        eprintln!("TUI 미설치. `mai setup` 으로 설치하세요.");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    // First-run check (except for setup/doctor themselves)
    match &cli.command {
        Commands::Setup | Commands::Doctor => {}
        _ => {
            if !is_setup_done() {
                eprintln!("⚠ mac-app-init 초기 설정이 필요합니다.");
                eprintln!("  mai setup  — 자동 업데이트 등록");
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
        println!("  mai install keyboard  — 도메인 설치");
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
        let status = if installed.contains(&name.as_str()) { "✓ installed" } else { "  available" };
        println!("{:<20} {}", name, status);
    }
}

fn domain_deps(name: &str) -> &'static [&'static str] {
    match name {
        "mount" => &["env"],
        _ => &[],
    }
}

fn record_installed_domain(name: &str, version: &str) {
    let mut reg = load_registry();
    if let Some(existing) = reg.installed.iter_mut().find(|d| d.name == name) {
        existing.version = version.to_string();
    } else {
        reg.installed.push(InstalledDomain {
            name: name.to_string(),
            version: version.to_string(),
        });
    }
    save_registry(&reg);
}

fn cmd_install(name: &str) {
    let mut reg = load_registry();
    if reg.installed.iter().any(|d| d.name == name) {
        println!("'{}' 이미 설치되어 있습니다. 업데이트: mai update {}", name, name);
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
            eprintln!("    mai install {}", d);
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
            run_post_install(name);
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
                    run_post_update(name);
                }
                Err(e) => eprintln!("✗ 업데이트 실패: {}", e),
            }
        }
        None => println!("'{}' 설치되어 있지 않습니다. 먼저: mai install {}", name, name),
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

    let asset = format!("mai-{}-apple-darwin.tar.gz", arch());
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
    let new_bin = dest_dir.join("mai");
    let current_bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/local/bin/mac"));

    if let Err(e) = fs::copy(&new_bin, &current_bin) {
        eprintln!("✗ 바이너리 교체 실패: {}", e);
        eprintln!("  수동: cp {} {}", new_bin.display(), current_bin.display());
        return;
    }

    // Cleanup
    fs::remove_file(&tar_path).ok();
    fs::remove_file(&new_bin).ok();

    println!("✓ mai 매니저 업데이트 완료 ({})", tag);
}

fn cmd_upgrade() {
    println!("=== 전체 업그레이드 ===\n");

    // 1. Self update
    println!("[1] mai 매니저 업데이트");
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
        eprintln!("  mai install {}", name);
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

fn run_domain_post_action(name: &str, args: &[&str], title: &str) {
    let bin = domain_bin_path(name);
    if !bin.exists() {
        return;
    }

    println!("→ {}: mai run {} {}", title, name, args.join(" "));
    let status = Command::new(&bin).args(args).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!(
                "⚠ {} 후속 작업 실패 (exit {}): mai run {} {}",
                name,
                s.code().unwrap_or(1),
                name,
                args.join(" ")
            );
        }
        Err(e) => {
            eprintln!(
                "⚠ {} 후속 작업 실행 실패 ({}): mai run {} {}",
                name,
                e,
                name,
                args.join(" ")
            );
        }
    }
}

fn run_post_install(name: &str) {
    match name {
        "tmux" => run_domain_post_action(name, &["setup"], "tmux 설치/초기화"),
        _ => {}
    }
}

fn run_post_update(name: &str) {
    match name {
        "tmux" => run_domain_post_action(name, &["install"], "tmux 도구 갱신"),
        _ => {}
    }
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
    println!("=== mac-app-init 셋업 ===\n");

    // 1. 디렉토리
    fs::create_dir_all(domains_dir()).ok();
    let cards_dir = PathBuf::from(home()).join(".mac-app-init/cards");
    fs::create_dir_all(&cards_dir).ok();
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = fs::set_permissions(&cards_dir, fs::Permissions::from_mode(0o700)); }
    println!("[1] ✓ 디렉토리 생성");

    // 2. TUI 설치 (domains 디렉터리에 — 사용자 PATH에 노출 안 함)
    let tui_path = domains_dir().join("mai-tui");
    if !tui_path.exists() {
        println!("[2] TUI 설치 중...");
        let target = if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" } else { "x86_64-apple-darwin" };
        let url = format!("https://github.com/{}/releases/latest/download/mai-tui-{}.tar.gz", GITHUB_REPO, target);
        let status = Command::new("bash")
            .args(["-c", &format!("curl -sfL '{}' | tar xz -C '{}'", url, domains_dir().display())])
            .status();
        if status.map(|s| s.success()).unwrap_or(false) {
            println!("    ✓ TUI 설치 완료");
        } else {
            println!("    ⚠ TUI 설치 실패 (mai upgrade 로 재시도)");
        }
    } else {
        println!("[2] ✓ TUI 이미 설치됨");
    }

    // 3. 핵심 도메인 설치 + registry 반영
    println!("[3] 핵심 도메인 확인...");
    let core = ["bootstrap", "env", "mount", "host", "cron", "shell", "keyboard", "git"];
    for name in &core {
        let already_installed = load_registry().installed.iter().any(|d| d.name == *name);
        if !domain_bin_path(name).exists() || !already_installed {
            print!("    {} 설치 중... ", name);
            match download_domain(name) {
                Ok(version) => {
                    record_installed_domain(name, &version);
                    println!("✓");
                }
                Err(e) => println!("⚠ {}", e),
            }
        }
    }
    println!("    ✓ 핵심 도메인 확인 완료");

    // 4. 의존성 (bootstrap)
    println!("[4] 의존성 확인...");
    let bootstrap_bin = domain_bin_path("bootstrap");
    if bootstrap_bin.exists() {
        let _ = Command::new(&bootstrap_bin).arg("install").status();
    }

    // 5. Scheduler LaunchAgent
    println!("[5] 자동 업데이트 등록...");
    match mac_host_core::cron::install_scheduler() {
        Ok(m) => println!("    {}", m),
        Err(e) => println!("    ⚠ {}", e),
    }
    let mut sched = mac_host_core::cron::load_schedule();
    if !sched.jobs.iter().any(|j| j.name == "mac-upgrade") {
        let mac_bin = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from(format!("{}/.local/bin/mac", home())));
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
            description: "매일 10시 mai + 도메인 자동 업데이트".into(),
        });
        let _ = mac_host_core::cron::save_schedule(&sched);
        println!("    ✓ mac-upgrade 작업 추가");
    }

    // 6. locale.json
    println!("[6] locale.json 생성...");
    let locale_path = PathBuf::from(home()).join(".mac-app-init/locale.json");
    if Command::new("nickel").arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
        // nickel 있으면 ncl export 시도
        let ncl_candidates = [
            PathBuf::from(home()).join(".mac-app-init/src/mac-app-init/ncl/domains.ncl"),
            PathBuf::from("ncl/domains.ncl"),
        ];
        for ncl in &ncl_candidates {
            if ncl.exists() {
                let out = Command::new("nickel").arg("export").arg(ncl).output();
                if let Ok(o) = out {
                    if o.status.success() {
                        let _ = fs::write(&locale_path, &o.stdout);
                        println!("    ✓ locale.json 생성 (ncl)");
                        break;
                    }
                }
            }
        }
    }
    if !locale_path.exists() {
        let _ = fs::write(&locale_path, "{\"domains\":{},\"dns_presets\":{},\"section_names\":{}}");
        println!("    ✓ locale.json fallback 생성");
    }

    println!("\n=== ✓ 셋업 완료 ===");
    println!("");
    println!("  mai-tui              TUI 실행");
    println!("  mai available        도메인 목록");
    println!("  mai install <name>   추가 도메인 설치");
    println!("  mai upgrade          전체 업그레이드");
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
        println!("  → mai setup");
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
        println!("  → mai run bootstrap install");
    }
}

// === Scheduler (core 위임 + deprecated 래퍼) ===

fn cmd_tick() {
    mac_host_core::cron::tick();
}

fn deprecated_notice(old: &str, new: &str) {
    eprintln!("⚠ `mac {}` 는 deprecated 입니다. `mai run cron {}` 를 사용하세요.", old, new);
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
