use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-git")]
#[command(about = "Git 프로필, SSH 키, GitHub CLI 설정 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 확인
    Status,
    /// Git 프로필 설정
    Profile {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        email: Option<String>,
    },
    /// SSH 키 생성/확인
    SshSetup,
    /// GitHub CLI 인증
    GhAuth,
    /// GitHub CLI 설치
    GhInstall,
    /// GitHub SSH 키 등록
    GhSshSetup,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn cmd_ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).output().map(|o| o.status.success()).unwrap_or(false)
}

fn cmd_stdout(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn git_config(key: &str) -> String {
    cmd_stdout("git", &["config", "--global", key])
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Profile { name, email } => cmd_profile(name, email),
        Commands::SshSetup => cmd_ssh_setup(),
        Commands::GhAuth => cmd_gh_auth(),
        Commands::GhInstall => cmd_gh_install(),
        Commands::GhSshSetup => cmd_gh_ssh_setup(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let name = git_config("user.name");
    let email = git_config("user.email");
    let ssh_key = PathBuf::from(home()).join(".ssh/id_ed25519");
    let ssh_key_exists = ssh_key.exists();
    let gh_installed = cmd_ok("gh", &["--version"]);
    let gh_authed = gh_installed && cmd_ok("gh", &["auth", "token"]);

    let spec = serde_json::json!({
        "tab": { "label_ko": "Git", "label": "Git", "icon": "🔱" },
        "group": "dev",        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    {
                        "key": "user.name",
                        "value": if name.is_empty() { "✗ 미설정".to_string() } else { format!("✓ {}", name) },
                        "status": if name.is_empty() { "error" } else { "ok" }
                    },
                    {
                        "key": "user.email",
                        "value": if email.is_empty() { "✗ 미설정".to_string() } else { format!("✓ {}", email) },
                        "status": if email.is_empty() { "error" } else { "ok" }
                    },
                    {
                        "key": "SSH 키 (id_ed25519)",
                        "value": if ssh_key_exists { "✓ 존재" } else { "✗ 없음" },
                        "status": if ssh_key_exists { "ok" } else { "error" }
                    },
                    {
                        "key": "GitHub CLI",
                        "value": if gh_installed { "✓ 설치됨" } else { "✗ 미설치" },
                        "status": if gh_installed { "ok" } else { "error" }
                    },
                    {
                        "key": "gh auth",
                        "value": if gh_authed { "✓ 인증됨" } else { "✗ 미인증" },
                        "status": if gh_authed { "ok" } else { "warn" }
                    }
                ]
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label_ko": "Git", "label": "Status (상태)", "command": "status", "key": "s" },
                    { "label_ko": "Git", "label": "SSH Setup (ed25519 키 생성)", "command": "ssh-setup", "key": "k" },
                    { "label_ko": "Git", "label": "gh Auth (GitHub 로그인)", "command": "gh-auth", "key": "a" },
                    { "label_ko": "Git", "label": "gh Install (gh CLI 설치)", "command": "gh-install", "key": "i" },
                    { "label_ko": "Git", "label": "gh SSH Setup (키 등록)", "command": "gh-ssh-setup", "key": "g" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}

fn cmd_status() {
    println!("=== Git 상태 ===\n");

    // Git version
    let git_ver = cmd_stdout("git", &["--version"]);
    println!("[git] {}", if git_ver.is_empty() { "✗ 미설치".into() } else { format!("✓ {}", git_ver) });

    // Profile
    let name = git_config("user.name");
    let email = git_config("user.email");
    println!("\n[프로필]");
    println!("  name:  {}", if name.is_empty() { "✗ 미설정" } else { &name });
    println!("  email: {}", if email.is_empty() { "✗ 미설정" } else { &email });

    // SSH keys
    println!("\n[SSH 키]");
    let ssh_dir = PathBuf::from(home()).join(".ssh");
    let key_types = ["id_ed25519", "id_rsa", "id_ecdsa"];
    let mut has_key = false;
    for kt in &key_types {
        let key = ssh_dir.join(kt);
        let pub_key = ssh_dir.join(format!("{}.pub", kt));
        if key.exists() {
            has_key = true;
            let fingerprint = cmd_stdout("ssh-keygen", &["-lf", &pub_key.to_string_lossy()]);
            let fp_short = fingerprint.split_whitespace().nth(1).unwrap_or("?");
            println!("  ✓ {} ({})", kt, fp_short);
        }
    }
    if !has_key {
        println!("  ✗ SSH 키 없음");
        println!("    → mac run git ssh-setup");
    }

    // SSH config
    let ssh_config = ssh_dir.join("config");
    println!("\n[SSH config]");
    if ssh_config.exists() {
        let content = fs::read_to_string(&ssh_config).unwrap_or_default();
        let hosts: Vec<&str> = content.lines()
            .filter(|l| l.trim().starts_with("Host "))
            .map(|l| l.trim().strip_prefix("Host ").unwrap_or(""))
            .collect();
        if hosts.is_empty() {
            println!("  ✓ 존재 (호스트 없음)");
        } else {
            for h in &hosts {
                println!("  ✓ {}", h);
            }
        }
    } else {
        println!("  ✗ 없음");
    }

    // GitHub CLI
    println!("\n[GitHub CLI]");
    let gh_ver = cmd_stdout("gh", &["--version"]);
    if gh_ver.is_empty() {
        println!("  ✗ gh 미설치");
        println!("    → mac run git gh-install");
    } else {
        println!("  ✓ {}", gh_ver.lines().next().unwrap_or(""));
        let auth = cmd_stdout("gh", &["auth", "status"]);
        if auth.contains("Logged in") || cmd_ok("gh", &["auth", "token"]) {
            let user = cmd_stdout("gh", &["api", "user", "-q", ".login"]);
            println!("  ✓ 인증됨 ({})", if user.is_empty() { "?" } else { &user });
        } else {
            println!("  ✗ 미인증");
            println!("    → mac run git gh-auth");
        }
    }

    // git-lfs
    println!("\n[Git LFS]");
    if cmd_ok("git", &["lfs", "version"]) {
        println!("  ✓ {}", cmd_stdout("git", &["lfs", "version"]));
    } else {
        println!("  ✗ 미설치 (brew install git-lfs)");
    }
}

fn cmd_profile(name: Option<String>, email: Option<String>) {
    if name.is_none() && email.is_none() {
        // Show current
        let n = git_config("user.name");
        let e = git_config("user.email");
        println!("현재 프로필:");
        println!("  name:  {}", if n.is_empty() { "(없음)" } else { &n });
        println!("  email: {}", if e.is_empty() { "(없음)" } else { &e });
        println!("\n설정: mac run git profile --name \"이름\" --email \"이메일\"");
        return;
    }
    if let Some(n) = &name {
        Command::new("git").args(["config", "--global", "user.name", n]).output().ok();
        println!("✓ user.name = {}", n);
    }
    if let Some(e) = &email {
        Command::new("git").args(["config", "--global", "user.email", e]).output().ok();
        println!("✓ user.email = {}", e);
    }
}

fn cmd_ssh_setup() {
    let ssh_dir = PathBuf::from(home()).join(".ssh");
    let key_path = ssh_dir.join("id_ed25519");

    if key_path.exists() {
        println!("✓ SSH 키 이미 존재: {}", key_path.display());
        let fp = cmd_stdout("ssh-keygen", &["-lf", &format!("{}.pub", key_path.display())]);
        println!("  {}", fp);
        return;
    }

    fs::create_dir_all(&ssh_dir).ok();
    let email = git_config("user.email");
    let comment = if email.is_empty() { "mac-app-init".to_string() } else { email };

    println!("SSH 키 생성 중...");
    let status = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-C", &comment, "-f", &key_path.to_string_lossy(), "-N", ""])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✓ SSH 키 생성 완료: {}", key_path.display());
            let pub_key = fs::read_to_string(format!("{}.pub", key_path.display())).unwrap_or_default();
            println!("\n공개 키:\n{}", pub_key.trim());
        }
        _ => println!("✗ SSH 키 생성 실패"),
    }
}

fn cmd_gh_install() {
    if cmd_ok("gh", &["--version"]) {
        println!("✓ gh 이미 설치됨");
        return;
    }
    println!("gh CLI 설치 중...");
    let status = Command::new("brew").args(["install", "gh"]).status();
    match status {
        Ok(s) if s.success() => println!("✓ gh 설치 완료"),
        _ => println!("✗ 설치 실패 (brew가 필요합니다)"),
    }
}

fn cmd_gh_auth() {
    if !cmd_ok("gh", &["--version"]) {
        println!("✗ gh CLI가 없습니다. 먼저: mac run git gh-install");
        return;
    }
    println!("GitHub 인증 시작 (브라우저가 열립니다)...");
    let _ = Command::new("gh").args(["auth", "login"]).status();
}

fn cmd_gh_ssh_setup() {
    let ssh_dir = PathBuf::from(home()).join(".ssh");
    let pub_key = ssh_dir.join("id_ed25519.pub");

    if !pub_key.exists() {
        println!("✗ SSH 키가 없습니다. 먼저: mac run git ssh-setup");
        return;
    }

    if !cmd_ok("gh", &["auth", "token"]) {
        println!("✗ GitHub 인증이 필요합니다. 먼저: mac run git gh-auth");
        return;
    }

    println!("GitHub에 SSH 키 등록 중...");
    let hostname = cmd_stdout("hostname", &["-s"]);
    let title = format!("mac-app-init ({})", if hostname.is_empty() { "mac" } else { &hostname });

    let status = Command::new("gh")
        .args(["ssh-key", "add", &pub_key.to_string_lossy(), "--title", &title])
        .status();

    match status {
        Ok(s) if s.success() => println!("✓ GitHub SSH 키 등록 완료"),
        _ => println!("✗ 등록 실패 (이미 등록되어 있을 수 있음)"),
    }
}
