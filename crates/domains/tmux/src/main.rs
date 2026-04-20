use clap::{Parser, Subcommand};
use mac_common::{
    cmd, paths,
    tui_spec::{self, TuiSpec},
};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO_URL: &str = "https://github.com/dalsoop/dalsoop-tmux-tools.git";

#[derive(Parser)]
#[command(name = "mac-domain-tmux")]
#[command(about = "tmux, TPM, dalsoop-tmux-tools 설치 및 tmux topbar 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 확인
    Status,
    /// tmux + dalsoop-tmux-tools 설치 후 초기화
    Setup,
    /// tmux + dalsoop-tmux-tools 설치/업데이트
    Install,
    /// tmux-sessionbar init 실행
    Init,
    /// .tmux.conf 재생성 + 적용
    Apply,
    /// tmux topbar TUI 실행
    Topbar,
    /// 계정별 설정 동기화
    Sync,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Setup => cmd_setup(),
        Commands::Install => cmd_install(),
        Commands::Init => cmd_init(),
        Commands::Apply => cmd_apply(),
        Commands::Topbar => cmd_topbar(),
        Commands::Sync => cmd_sync(),
        Commands::TuiSpec => print_tui_spec(),
    }
}


fn local_bin_dir() -> PathBuf {
    PathBuf::from(paths::home()).join(".local/bin")
}

fn tmux_source_dir() -> PathBuf {
    paths::base().join("src/dalsoop-tmux-tools")
}

fn tool_bin(name: &str) -> PathBuf {
    local_bin_dir().join(name)
}

fn sessionbar_bin() -> PathBuf {
    tool_bin("tmux-sessionbar")
}

fn windowbar_bin() -> PathBuf {
    tool_bin("tmux-windowbar")
}

fn topbar_bin() -> PathBuf {
    tool_bin("tmux-topbar")
}

fn compat_bin() -> PathBuf {
    tool_bin("tmux-config")
}

fn tmux_conf_path() -> PathBuf {
    PathBuf::from(paths::home()).join(".tmux.conf")
}

fn tpm_dir() -> PathBuf {
    PathBuf::from(paths::home()).join(".tmux/plugins/tpm")
}

fn sessionbar_config() -> PathBuf {
    PathBuf::from(paths::home()).join(".config/tmux-sessionbar/config.toml")
}

fn windowbar_config() -> PathBuf {
    PathBuf::from(paths::home()).join(".config/tmux-windowbar/config.toml")
}



fn ensure_shell_integration() {
    let shell_bin = paths::domains_dir().join("mac-domain-shell");
    let local_bin = local_bin_dir().display().to_string();

    // PATH 추가 (shell 도메인 CLI 호출)
    let _ = Command::new(&shell_bin)
        .args(["path", "add", &local_bin, "--label", "tmux tools"])
        .output();

    // alias 등록
    let aliases = [
        ("mts", "mai run tmux status"),
        ("mti", "mai run tmux init"),
        ("mta", "mai run tmux apply"),
        ("mtt", "mai run tmux topbar"),
    ];
    for (name, command) in aliases {
        let _ = Command::new(&shell_bin)
            .args(["alias", "add", name, command])
            .output();
    }

    // shell.sh 동기화
    let _ = Command::new(&shell_bin).arg("sync").output();
}

fn prefixed_path() -> OsString {
    let mut dirs = vec![local_bin_dir()];
    dirs.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(dirs).unwrap_or_else(|_| std::env::var_os("PATH").unwrap_or_default())
}

fn run_with_local_bin(bin: &Path, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .env("PATH", prefixed_path())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn command_version(bin: &Path) -> String {
    Command::new(bin)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.lines().next().unwrap_or("").trim().to_string()
        })
        .unwrap_or_default()
}

fn mark(ok: bool) -> &'static str {
    if ok { "✓" } else { "✗" }
}

fn ensure_brew() -> bool {
    if cmd::ok("brew", &["--version"]) {
        true
    } else {
        eprintln!("✗ Homebrew가 필요합니다. 먼저 `mai run bootstrap install` 또는 brew 설치를 진행하세요.");
        false
    }
}

fn ensure_rust() -> bool {
    let ok = cmd::ok("cargo", &["--version"]) && cmd::ok("rustc", &["--version"]);
    if !ok {
        eprintln!("✗ Rust/Cargo가 필요합니다. 먼저 `mai run bootstrap install`을 실행하세요.");
    }
    ok
}

fn ensure_git() -> bool {
    if cmd::ok("git", &["--version"]) {
        return true;
    }
    if !ensure_brew() {
        return false;
    }
    println!("[git] 설치 중...");
    Command::new("brew")
        .args(["install", "git"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn ensure_tmux() -> bool {
    if cmd::ok("tmux", &["-V"]) {
        return true;
    }
    if !ensure_brew() {
        return false;
    }
    println!("[tmux] 설치 중...");
    Command::new("brew")
        .args(["install", "tmux"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn clone_or_update_repo() -> bool {
    let repo_dir = tmux_source_dir();
    if repo_dir.join(".git").exists() {
        println!("[repo] 업데이트 중...");
        return Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["pull", "--ff-only"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    if let Some(parent) = repo_dir.parent() {
        let _ = fs::create_dir_all(parent);
    }
    println!("[repo] clone 중...");
    Command::new("git")
        .args(["clone", "--depth", "1", REPO_URL])
        .arg(&repo_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn build_tools() -> bool {
    println!("[build] dalsoop-tmux-tools 빌드 중...");
    Command::new("cargo")
        .current_dir(tmux_source_dir())
        .args([
            "build",
            "--release",
            "--locked",
            "-p",
            "tmux-sessionbar",
            "-p",
            "tmux-windowbar",
            "-p",
            "tmux-config",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn install_compiled_binaries() -> bool {
    let release_dir = tmux_source_dir().join("target/release");
    let pairs = [
        ("tmux-sessionbar", sessionbar_bin()),
        ("tmux-windowbar", windowbar_bin()),
        ("tmux-topbar", topbar_bin()),
    ];

    let _ = fs::create_dir_all(local_bin_dir());

    for (name, dst) in pairs {
        let src = release_dir.join(name);
        if !src.exists() {
            eprintln!("✗ 빌드 산출물이 없습니다: {}", src.display());
            return false;
        }
        if fs::copy(&src, &dst).is_err() {
            eprintln!("✗ 바이너리 복사 실패: {}", dst.display());
            return false;
        }
        if let Ok(meta) = fs::metadata(&dst) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&dst, perms);
        }
    }

    let compat = compat_bin();
    if fs::symlink_metadata(&compat).is_ok() {
        let _ = fs::remove_file(&compat);
    }
    symlink(topbar_bin(), &compat).is_ok() || fs::copy(topbar_bin(), compat).is_ok()
}

fn ensure_installed() -> bool {
    sessionbar_bin().exists() && windowbar_bin().exists() && topbar_bin().exists()
}

fn cmd_setup() {
    if !cmd_install_inner() {
        std::process::exit(1);
    }
    if !cmd_init_inner() {
        std::process::exit(1);
    }
}

fn cmd_install() {
    if !cmd_install_inner() {
        std::process::exit(1);
    }
}

fn cmd_install_inner() -> bool {
    println!("=== tmux 환경 설치 ===\n");

    if !ensure_brew() || !ensure_git() || !ensure_rust() {
        return false;
    }

    if ensure_tmux() {
        println!("[tmux] {}", cmd::stdout("tmux", &["-V"]));
    } else {
        eprintln!("✗ tmux 설치 실패");
        return false;
    }

    if !clone_or_update_repo() {
        eprintln!("✗ dalsoop-tmux-tools 소스 동기화 실패");
        return false;
    }
    if !build_tools() {
        eprintln!("✗ dalsoop-tmux-tools 빌드 실패");
        return false;
    }
    if !install_compiled_binaries() {
        eprintln!("✗ tmux-tools 바이너리 설치 실패");
        return false;
    }

    ensure_shell_integration();

    println!("\n=== 완료 ===");
    println!("  설치 위치: {}", local_bin_dir().display());
    println!("  shell 연동: ~/.local/bin + mts/mti/mta/mtt alias 등록");
    println!("  다음 단계: mai run tmux init");
    true
}

fn cmd_init() {
    if !cmd_init_inner() {
        std::process::exit(1);
    }
}

fn cmd_init_inner() -> bool {
    if !ensure_installed() {
        eprintln!("✗ tmux-tools가 아직 설치되지 않았습니다. 먼저 `mai run tmux install`을 실행하세요.");
        return false;
    }
    println!("=== tmux 초기화 ===\n");
    if run_with_local_bin(&sessionbar_bin(), &["init"]) {
        println!("\n✓ tmux topbar 초기화 완료");
        true
    } else {
        eprintln!("✗ tmux-sessionbar init 실패");
        false
    }
}

fn cmd_apply() {
    if !ensure_installed() {
        eprintln!("✗ tmux-tools가 아직 설치되지 않았습니다. 먼저 `mai run tmux install`을 실행하세요.");
        std::process::exit(1);
    }
    if !run_with_local_bin(&sessionbar_bin(), &["apply"]) {
        eprintln!("✗ tmux-sessionbar apply 실패");
        std::process::exit(1);
    }
}

fn cmd_topbar() {
    if !ensure_installed() {
        eprintln!("✗ tmux-tools가 아직 설치되지 않았습니다. 먼저 `mai run tmux install`을 실행하세요.");
        std::process::exit(1);
    }
    if !run_with_local_bin(&topbar_bin(), &[]) {
        eprintln!("✗ tmux-topbar 실행 실패");
        std::process::exit(1);
    }
}

fn cmd_sync() {
    if !ensure_installed() {
        eprintln!("✗ tmux-tools가 아직 설치되지 않았습니다. 먼저 `mai run tmux install`을 실행하세요.");
        std::process::exit(1);
    }
    if !run_with_local_bin(&sessionbar_bin(), &["sync"]) {
        eprintln!("✗ tmux-sessionbar sync 실패");
        std::process::exit(1);
    }
}

fn cmd_status() {
    let has_tmux = cmd::ok("tmux", &["-V"]);
    let has_tpm = tpm_dir().exists();
    let has_sessionbar = sessionbar_bin().exists();
    let has_windowbar = windowbar_bin().exists();
    let has_topbar = topbar_bin().exists();
    let has_compat = compat_bin().exists();
    let has_source = tmux_source_dir().join(".git").exists();
    let has_tmux_conf = tmux_conf_path().exists();
    let has_session_cfg = sessionbar_config().exists();
    let has_window_cfg = windowbar_config().exists();
    let tmux_running = Command::new("tmux")
        .arg("list-sessions")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    println!("=== tmux 상태 ===\n");
    println!("[core]");
    println!("  {} tmux {}", mark(has_tmux), if has_tmux { cmd::stdout("tmux", &["-V"]) } else { "미설치".into() });
    println!("  {} TPM {}", mark(has_tpm), if has_tpm { tpm_dir().display().to_string() } else { "미설치".into() });

    println!("\n[binaries]");
    println!("  {} tmux-sessionbar {}", mark(has_sessionbar), version_or_missing(&sessionbar_bin()));
    println!("  {} tmux-windowbar {}", mark(has_windowbar), version_or_missing(&windowbar_bin()));
    println!("  {} tmux-topbar {}", mark(has_topbar), version_or_missing(&topbar_bin()));
    println!("  {} tmux-config {}", mark(has_compat), if has_compat { compat_bin().display().to_string() } else { "미설치".into() });

    println!("\n[config]");
    println!("  {} source repo {}", mark(has_source), tmux_source_dir().display());
    println!("  {} .tmux.conf {}", mark(has_tmux_conf), tmux_conf_path().display());
    println!("  {} sessionbar config {}", mark(has_session_cfg), sessionbar_config().display());
    println!("  {} windowbar config {}", mark(has_window_cfg), windowbar_config().display());
    println!("  {} tmux server {}", mark(tmux_running), if tmux_running { "실행 중" } else { "미실행" });

    if !(has_tmux && has_sessionbar && has_windowbar && has_topbar) {
        println!("\n→ 설치: mai run tmux setup");
    } else if !(has_session_cfg && has_window_cfg && has_tmux_conf) {
        println!("\n→ 초기화: mai run tmux init");
    } else {
        println!("\n→ topbar 실행: mai run tmux topbar");
    }
}

fn version_or_missing(path: &Path) -> String {
    if !path.exists() {
        return "미설치".into();
    }
    let ver = command_version(path);
    if ver.is_empty() {
        path.display().to_string()
    } else {
        ver
    }
}

fn print_tui_spec() {
    let has_tmux = cmd::ok("tmux", &["-V"]);
    let has_tpm = tpm_dir().exists();
    let has_sessionbar = sessionbar_bin().exists();
    let has_windowbar = windowbar_bin().exists();
    let has_topbar = topbar_bin().exists();
    let has_source = tmux_source_dir().join(".git").exists();
    let has_tmux_conf = tmux_conf_path().exists();
    let has_session_cfg = sessionbar_config().exists();
    let has_window_cfg = windowbar_config().exists();

    let ready = has_tmux
        && has_tpm
        && has_sessionbar
        && has_windowbar
        && has_topbar
        && has_tmux_conf
        && has_session_cfg
        && has_window_cfg;

    let summary = if ready {
        "tmux topbar 사용 가능"
    } else if has_sessionbar || has_windowbar || has_topbar {
        "설치됨, 초기화 필요"
    } else {
        "미설치"
    };

    TuiSpec::new("tmux")
        .refresh(30)
        .usage(ready, summary)
        .kv(
            "상태",
            vec![
                tui_spec::kv_item(
                    "tmux",
                    if has_tmux { "✓ 설치됨" } else { "✗ 미설치" },
                    if has_tmux { "ok" } else { "error" },
                ),
                tui_spec::kv_item(
                    "TPM",
                    if has_tpm { "✓ 설치됨" } else { "✗ 미설치" },
                    if has_tpm { "ok" } else { "warn" },
                ),
                tui_spec::kv_item(
                    "tmux-sessionbar",
                    if has_sessionbar { "✓ 설치됨" } else { "✗ 미설치" },
                    if has_sessionbar { "ok" } else { "error" },
                ),
                tui_spec::kv_item(
                    "tmux-windowbar",
                    if has_windowbar { "✓ 설치됨" } else { "✗ 미설치" },
                    if has_windowbar { "ok" } else { "error" },
                ),
                tui_spec::kv_item(
                    "tmux-topbar",
                    if has_topbar { "✓ 설치됨" } else { "✗ 미설치" },
                    if has_topbar { "ok" } else { "error" },
                ),
            ],
        )
        .kv(
            "설정",
            vec![
                tui_spec::kv_item(
                    "소스 동기화",
                    if has_source { "✓ 준비됨" } else { "✗ 없음" },
                    if has_source { "ok" } else { "warn" },
                ),
                tui_spec::kv_item(
                    ".tmux.conf",
                    if has_tmux_conf { "✓ 생성됨" } else { "✗ 미생성" },
                    if has_tmux_conf { "ok" } else { "warn" },
                ),
                tui_spec::kv_item(
                    "sessionbar config",
                    if has_session_cfg { "✓ 생성됨" } else { "✗ 미생성" },
                    if has_session_cfg { "ok" } else { "warn" },
                ),
                tui_spec::kv_item(
                    "windowbar config",
                    if has_window_cfg { "✓ 생성됨" } else { "✗ 미생성" },
                    if has_window_cfg { "ok" } else { "warn" },
                ),
            ],
        )
        .buttons()
        .text(
            "안내",
            "권장 순서:\n  mai run tmux setup\n  mai run tmux topbar\n\n소스 빌드가 필요하므로 Rust/Cargo가 없으면 먼저 `mai run bootstrap install`을 실행하세요.",
        )
        .print();
}
