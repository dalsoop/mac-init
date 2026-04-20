use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-bootstrap")]
#[command(about = "mac-app-init 최초 설치 — brew, gh, dotenvx, rust 의존성")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 의존성 상태 확인
    Status,
    /// 전체 의존성 설치
    Install,
    /// 누락된 것만 설치
    Check,
    /// PATH + shell.sh source 설정 (초기 셋업 or 재설정)
    SetupPath,
    /// SD 백업 초기 설정 (경로 + 자동백업 + 자동추출)
    SetupSd,
    /// 전체 초기 셋업 (의존성 + PATH + SD)
    SetupAll,
    /// 시스템 LaunchAgent 관리
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

#[derive(Subcommand)]
enum AgentAction {
    /// 전체 LaunchAgent 목록
    List,
    /// 상세 정보
    Info { label: String },
    /// 로드 (시작)
    Load { label: String },
    /// 언로드 (정지)
    Unload { label: String },
    /// 재시작
    Restart { label: String },
    /// 로그 확인
    Logs { label: String },
}

struct Dep {
    name: &'static str,
    check_cmd: &'static str,
    check_args: &'static [&'static str],
    install_steps: &'static [(&'static str, &'static [&'static str])],
    description: &'static str,
}

const DEPS: &[Dep] = &[
    Dep {
        name: "Homebrew",
        check_cmd: "brew",
        check_args: &["--version"],
        install_steps: &[
            ("bash", &["-c", "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""]),
        ],
        description: "macOS 패키지 매니저",
    },
    Dep {
        name: "Rust",
        check_cmd: "rustc",
        check_args: &["--version"],
        install_steps: &[
            ("bash", &["-c", "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"]),
        ],
        description: "Rust 컴파일러 + Cargo",
    },
    Dep {
        name: "GitHub CLI",
        check_cmd: "gh",
        check_args: &["--version"],
        install_steps: &[
            ("brew", &["install", "gh"]),
        ],
        description: "GitHub CLI (mac install에 필요)",
    },
    Dep {
        name: "dotenvx",
        check_cmd: "dotenvx",
        check_args: &["--version"],
        install_steps: &[
            ("brew", &["install", "dotenvx/brew/dotenvx"]),
        ],
        description: ".env 암호화 (connect에 필요)",
    },
    Dep {
        name: "Nickel",
        check_cmd: "nickel",
        check_args: &["--version"],
        install_steps: &[
            ("brew", &["install", "nickel"]),
        ],
        description: "설정 스키마 언어",
    },
    Dep {
        name: "WireGuard",
        check_cmd: "wg",
        check_args: &["--version"],
        install_steps: &[
            ("brew", &["install", "wireguard-tools"]),
        ],
        description: "VPN CLI (wireguard 도메인에 필요)",
    },
];

fn check_installed(dep: &Dep) -> Option<String> {
    Command::new(dep.check_cmd)
        .args(dep.check_args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
}

fn install_dep(dep: &Dep) -> bool {
    for (cmd, args) in dep.install_steps {
        println!("  → {} {}", cmd, args.join(" "));
        let status = Command::new(cmd)
            .args(*args)
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => return false,
        }
    }
    true
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Install => cmd_install(),
        Commands::Check => cmd_check(),
        Commands::SetupPath => cmd_setup_path(),
        Commands::SetupSd => cmd_setup_sd(),
        Commands::SetupAll => { cmd_install(); cmd_setup_path(); cmd_setup_sd(); },
        Commands::Agent { action } => match action {
            AgentAction::List => mac_host_core::cron::list(),
            AgentAction::Info { label } => mac_host_core::cron::info(&label),
            AgentAction::Load { label } => mac_host_core::cron::load(&label),
            AgentAction::Unload { label } => mac_host_core::cron::unload(&label),
            AgentAction::Restart { label } => mac_host_core::cron::restart(&label),
            AgentAction::Logs { label } => mac_host_core::cron::logs(&label),
        },
        Commands::TuiSpec => print_tui_spec(),
    }
}

use mac_common::{paths, tui_spec::{self, TuiSpec}};

fn home() -> String { paths::home() }

fn shell_store_path() -> std::path::PathBuf {
    std::path::PathBuf::from(home()).join(".mac-app-init/shell.json")
}

fn shell_sh_path() -> std::path::PathBuf {
    std::path::PathBuf::from(home()).join(".mac-app-init/shell.sh")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShellPathEntry {
    path: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    label: String,
}

fn default_true() -> bool { true }

#[derive(Debug, Default, Serialize, Deserialize)]
struct ShellStore {
    #[serde(default)]
    paths: Vec<ShellPathEntry>,
    #[serde(default)]
    aliases: std::collections::BTreeMap<String, String>,
}

fn load_shell_store() -> ShellStore {
    let p = shell_store_path();
    if !p.exists() { return ShellStore::default(); }
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_default()).unwrap_or_default()
}

fn save_shell_store(s: &ShellStore) {
    let p = shell_store_path();
    if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
    let _ = std::fs::write(&p, serde_json::to_string_pretty(s).unwrap_or_default());
}

fn generate_shell_sh(s: &ShellStore) {
    let mut lines = vec![
        "#!/bin/sh".into(),
        "# mac-app-init shell — 자동 생성. 직접 수정 금지.".into(),
        "# mac run shell path/alias 로 관리.".into(),
        String::new(),
        "# === PATH ===".into(),
    ];

    for e in &s.paths {
        if e.enabled {
            let c = if e.label.is_empty() { String::new() } else { format!("  # {}", e.label) };
            lines.push(format!("export PATH=\"{}:$PATH\"{}", paths::expand(&e.path), c));
        }
    }

    lines.push(String::new());
    lines.push("# === Aliases ===".into());
    for (name, cmd) in &s.aliases {
        lines.push(format!("alias {}='{}'", name, cmd.replace('\'', "'\\''")));
    }

    let sh = shell_sh_path();
    if let Some(parent) = sh.parent() { let _ = std::fs::create_dir_all(parent); }
    let _ = std::fs::write(&sh, lines.join("\n") + "\n");
}

fn cmd_setup_path() {
    use std::fs;
    use std::path::PathBuf;

    let domains_dir = format!("{}/.mac-app-init/domains", home());
    let local_bin_dir = format!("{}/.local/bin", home());
    let shell_sh = shell_sh_path();
    let zshrc = PathBuf::from(home()).join(".zshrc");

    println!("=== PATH + shell 셋업 ===\n");

    let mut content = fs::read_to_string(&zshrc).unwrap_or_default();
    let mut changed = false;

    let filtered_lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.contains(".mac-app-init/aliases.sh"))
        .collect();
    let filtered = filtered_lines.join("\n");
    if filtered != content {
        content = filtered;
        if !content.ends_with('\n') { content.push('\n'); }
        changed = true;
        println!("✓ legacy aliases.sh source 제거");
    }

    if !content.contains(".mac-app-init/shell.sh") {
        if !content.ends_with('\n') { content.push('\n'); }
        content.push_str(&format!("\n# mac-app-init shell\nsource {}\n", shell_sh.display()));
        changed = true;
        println!("✓ source shell.sh 추가");
    } else {
        println!("✓ source shell.sh 이미 등록됨");
    }

    if changed {
        if let Err(e) = fs::write(&zshrc, &content) {
            eprintln!("✗ ~/.zshrc 쓰기 실패: {}", e);
            return;
        }
    }

    let mut store = load_shell_store();
    let wanted_paths = [
        (domains_dir.as_str(), "mac domains"),
        (local_bin_dir.as_str(), "local user bin"),
    ];
    for (path, label) in wanted_paths {
        if let Some(entry) = store.paths.iter_mut().find(|e| e.path == path) {
            entry.enabled = true;
            if entry.label.is_empty() {
                entry.label = label.into();
            }
        } else {
            store.paths.push(ShellPathEntry {
                path: path.into(),
                enabled: true,
                label: label.into(),
            });
        }
    }
    save_shell_store(&store);
    generate_shell_sh(&store);
    println!("✓ shell.json / shell.sh 동기화");

    println!("\n새 터미널 열면 적용됩니다.");
    println!("  mac, mac-domain-*, ~/.local/bin 도구가 바로 실행 가능해집니다.");
}

fn cmd_setup_sd() {
    use std::fs;
    use std::path::PathBuf;

    println!("=== SD 백업 초기 설정 ===\n");

    let sd_bin = PathBuf::from(home()).join(".mac-app-init/domains/mac-domain-sd-backup");
    if !sd_bin.exists() {
        eprintln!("✗ sd-backup 도메인 미설치. `mac install sd-backup` 먼저.");
        return;
    }

    // 1. 로컬 백업 경로
    let backup_dir = format!("{}/Documents/WORK/미디어/SD백업", home());
    let _ = fs::create_dir_all(&backup_dir);
    let _ = Command::new(&sd_bin).args(["set-target", &backup_dir]).status();

    // 2. 자동 백업 on
    let _ = Command::new(&sd_bin).args(["auto", "on"]).status();

    // 3. 자동 추출 on
    let _ = Command::new(&sd_bin).args(["eject", "on"]).status();

    println!("\n=== SD 백업 설정 완료 ===");
    println!("  로컬 경로: {}", backup_dir);
    println!("  자동 백업: ✓ (30초마다 스캔)");
    println!("  자동 추출: ✓ (백업 후 SD 안전 추출)");
    println!("  NAS 동기화: 꺼짐 (LAN 환경에서 `mac run sd-backup sync on`)");
}

fn print_tui_spec() {
    let items: Vec<serde_json::Value> = DEPS.iter().map(|dep| {
        let ver = check_installed(dep);
        let (value, status) = match &ver {
            Some(v) => (format!("✓ {}", v), "ok"),
            None => ("✗ 미설치".to_string(), "error"),
        };
        tui_spec::kv_item(dep.name, &value, status)
    }).collect();

    // PATH 설정 상태
    use std::path::PathBuf;
    let shell_sh = PathBuf::from(home()).join(".mac-app-init/shell.sh");
    let zshrc = std::fs::read_to_string(PathBuf::from(home()).join(".zshrc")).unwrap_or_default();
    let path_ok = shell_sh.exists() && zshrc.contains(".mac-app-init/shell.sh");

    // SD 백업 상태
    let sd_cfg_path = PathBuf::from(home()).join(".mac-app-init/sd-backup.json");
    let (sd_auto, sd_eject, sd_target) = if sd_cfg_path.exists() {
        let s = std::fs::read_to_string(&sd_cfg_path).unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
        (
            v.get("auto_enabled").and_then(|v| v.as_bool()).unwrap_or(false),
            v.get("auto_eject").and_then(|v| v.as_bool()).unwrap_or(false),
            v.get("backup_target").and_then(|v| v.as_str()).unwrap_or("미설정").to_string(),
        )
    } else { (false, false, "미설정".into()) };

    // TCC 상태
    let mac_bin_path = format!("{}/.cargo/bin/mac", home());
    let tcc_ok = Command::new("sqlite3")
        .args([
            &format!("{}/Library/Application Support/com.apple.TCC/TCC.db", home()),
            &format!("SELECT auth_value FROM access WHERE client='{}' AND service='kTCCServiceSystemPolicyDocumentsFolder';", mac_bin_path),
        ])
        .output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "2")
        .unwrap_or(false);

    let installed_count = items.iter().filter(|i| i.get("status").and_then(|s| s.as_str()) == Some("ok")).count();
    let total_count = DEPS.len();
    let usage_active = installed_count == total_count;
    let usage_summary = format!("{}/{} 설치됨", installed_count, total_count);

    TuiSpec::new("bootstrap")
        .refresh(60)
        .usage(usage_active, &usage_summary)
        .kv("상태", items)
        .kv("초기 설정", vec![
            tui_spec::kv_item("PATH (shell.sh)",
                if path_ok { "✓ 설정됨" } else { "✗ 미설정" },
                if path_ok { "ok" } else { "error" }),
            tui_spec::kv_item("TCC (Documents 접근)",
                if tcc_ok { "✓ 허용" } else { "✗ 미허용" },
                if tcc_ok { "ok" } else { "warn" }),
            tui_spec::kv_item("SD 자동 백업",
                if sd_auto { "✓ 켜짐" } else { "꺼짐" },
                if sd_auto { "ok" } else { "warn" }),
            tui_spec::kv_item("SD 자동 추출",
                if sd_eject { "✓ 켜짐" } else { "꺼짐" },
                if sd_eject { "ok" } else { "warn" }),
            tui_spec::kv_item("SD 백업 경로", &sd_target,
                if sd_target == "미설정" { "error" } else { "ok" }),
        ])
        .buttons()
        .print();
}

fn cmd_status() {
    println!("=== 의존성 상태 ===\n");

    let mut ok = 0;
    let mut missing = 0;

    for dep in DEPS {
        match check_installed(dep) {
            Some(ver) => {
                println!("  ✓ {:<15} {} ({})", dep.name, ver, dep.description);
                ok += 1;
            }
            None => {
                println!("  ✗ {:<15} 미설치 ({})", dep.name, dep.description);
                missing += 1;
            }
        }
    }

    println!("\n  {ok}개 설치됨, {missing}개 누락");

    // TCC 상태 점검 — mac CLI 가 Documents 접근 허용됐는지
    println!("\n=== 보안 (TCC) ===\n");
    let mac_bin = std::env::var("HOME").unwrap_or_default() + "/.cargo/bin/mac";
    let tcc_check = Command::new("sqlite3")
        .args([
            &format!("{}/Library/Application Support/com.apple.TCC/TCC.db", std::env::var("HOME").unwrap_or_default()),
            &format!("SELECT auth_value FROM access WHERE client='{}' AND service='kTCCServiceSystemPolicyDocumentsFolder';", mac_bin),
        ])
        .output();
    let mac_allowed = tcc_check.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "2")
        .unwrap_or(false);
    println!("  {} mac CLI Documents 접근: {}",
        if mac_allowed { "✓" } else { "✗" },
        if mac_allowed { "허용됨" } else { "미허용 — 터미널에서 mac 실행 시 '접근 허용' 팝업에 허용 필요" }
    );
    println!("  ℹ 도메인 바이너리는 `mac run` 경유로 실행하면 TCC 상속됨.");
    if missing > 0 {
        println!("  → mac run bootstrap install");
    }
}

fn cmd_check() {
    println!("=== 누락된 의존성 확인 ===\n");

    let mut installed_count = 0;
    for dep in DEPS {
        if check_installed(dep).is_some() {
            continue;
        }
        println!("[{}] {} 설치 중...", dep.name, dep.description);
        if install_dep(dep) {
            println!("  ✓ {} 설치 완료", dep.name);
            installed_count += 1;
        } else {
            println!("  ✗ {} 설치 실패", dep.name);
        }
    }

    if installed_count == 0 {
        println!("  모든 의존성이 이미 설치되어 있습니다. ✓");
    } else {
        println!("\n  {}개 설치 완료", installed_count);
    }
}

fn cmd_install() {
    println!("=== 전체 의존성 설치 ===\n");

    for dep in DEPS {
        match check_installed(dep) {
            Some(ver) => {
                println!("  ✓ {:<15} 이미 설치됨 ({})", dep.name, ver);
            }
            None => {
                println!("  ⏳ {:<15} 설치 중...", dep.name);
                if install_dep(dep) {
                    println!("  ✓ {:<15} 설치 완료", dep.name);
                } else {
                    println!("  ✗ {:<15} 설치 실패", dep.name);
                }
            }
        }
    }

    // .env 초기화
    let env_path = format!("{}/.env", std::env::var("HOME").unwrap_or_default());
    if !std::path::Path::new(&env_path).exists() {
        println!("\n  .env 파일 생성 중...");
        let example = include_str!("../../../../example.env");
        std::fs::write(&env_path, example).ok();
        println!("  ✓ ~/.env 생성 완료");
        println!("  → 필요한 값을 설정 후 dotenvx encrypt 실행");
    }

    println!("\n=== 완료 ===");
    println!("  mac available     — 사용 가능한 도메인");
    println!("  mac install cron  — 도메인 설치");
}
