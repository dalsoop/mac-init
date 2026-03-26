use std::path::Path;
use std::process::Command;

use crate::common;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/jeonghan".to_string())
}

pub fn status() {
    println!("=== 작업 환경 상태 ===\n");

    // 셸
    println!("[셸] zsh");

    // p10k
    let (has_p10k_brew, _) = common::run_cmd_quiet("brew", &["list", "powerlevel10k"]);
    println!("[Powerlevel10k] {}", if has_p10k_brew { "✓ 설치됨" } else { "✗ 미설치" });

    // tmux
    let (has_tmux, _) = common::run_cmd_quiet("which", &["tmux"]);
    if has_tmux {
        let (_, ver) = common::run_cmd_quiet("tmux", &["-V"]);
        println!("[tmux] ✓ {}", ver.trim());
    } else {
        println!("[tmux] ✗ 미설치");
    }
    let has_tmux_conf = Path::new(&format!("{}/.tmux.conf", home())).exists();
    println!("[tmux.conf] {}", if has_tmux_conf { "✓" } else { "✗" });
    let has_tpm = Path::new(&format!("{}/.tmux/plugins/tpm", home())).exists();
    println!("[TPM] {}", if has_tpm { "✓ 설치됨" } else { "✗ 미설치" });

    // 개발 도구
    println!("\n[CLI 도구]");
    let tools = [
        ("bat", "cat 대체"),
        ("eza", "ls 대체"),
        ("fzf", "퍼지 파인더"),
        ("fd", "find 대체"),
        ("ripgrep", "grep 대체"),
        ("lazygit", "Git TUI"),
        ("jq", "JSON 파서"),
        ("htop", "프로세스 모니터"),
        ("neovim", "에디터"),
        ("starship", "셸 프롬프트"),
    ];

    for (tool, desc) in tools {
        let (ok, _) = common::run_cmd_quiet("brew", &["list", tool]);
        let mark = if ok { "✓" } else { "✗" };
        println!("  {mark} {tool} — {desc}");
    }

    // Node.js / Rust / Python
    println!("\n[런타임]");
    for (cmd, name) in [("node", "Node.js"), ("rustc", "Rust"), ("python3", "Python"), ("bun", "Bun")] {
        let (ok, ver) = common::run_cmd_quiet(cmd, &["--version"]);
        if ok {
            println!("  ✓ {name} {}", ver.trim());
        } else {
            println!("  ✗ {name}");
        }
    }
}

pub fn install_tmux() {
    // tmux
    let (has_tmux, _) = common::run_cmd_quiet("which", &["tmux"]);
    if has_tmux {
        println!("[workspace] tmux 이미 설치됨");
    } else {
        println!("[workspace] tmux 설치 중...");
        let ok = Command::new("brew")
            .args(["install", "tmux"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("[workspace] tmux 설치 실패");
            std::process::exit(1);
        }
        println!("[workspace] tmux 설치 완료");
    }

    // TPM (Tmux Plugin Manager)
    let tpm_path = format!("{}/.tmux/plugins/tpm", home());
    if Path::new(&tpm_path).exists() {
        println!("[workspace] TPM 이미 설치됨");
    } else {
        println!("[workspace] TPM 설치 중...");
        let (ok, _, _) = common::run_cmd("git", &[
            "clone", "https://github.com/tmux-plugins/tpm", &tpm_path,
        ]);
        if ok {
            println!("[workspace] TPM 설치 완료");
            println!("  tmux 실행 후 prefix + I 로 플러그인 설치");
        }
    }
}

pub fn install_tools() {
    let tools = [
        "bat",
        "eza",
        "fzf",
        "fd",
        "ripgrep",
        "lazygit",
        "jq",
        "htop",
    ];

    println!("[workspace] CLI 도구 설치 중...\n");

    for tool in tools {
        let (installed, _) = common::run_cmd_quiet("brew", &["list", tool]);
        if installed {
            println!("  ✓ {tool} 이미 설치됨");
        } else {
            print!("  ⏳ {tool} 설치 중... ");
            let ok = Command::new("brew")
                .args(["install", tool])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            println!("{}", if ok { "✓" } else { "✗ 실패" });
        }
    }

    println!("\n[workspace] CLI 도구 설치 완료");
}

pub fn setup_shell() {
    println!("[workspace] 셸 환경 설정 중...\n");

    // Powerlevel10k
    let (has_p10k, _) = common::run_cmd_quiet("brew", &["list", "powerlevel10k"]);
    if has_p10k {
        println!("  ✓ Powerlevel10k 이미 설치됨");
    } else {
        print!("  ⏳ Powerlevel10k 설치 중... ");
        let ok = Command::new("brew")
            .args(["install", "powerlevel10k"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        println!("{}", if ok { "✓" } else { "✗ 실패" });
    }

    // zsh-autosuggestions
    let (has_auto, _) = common::run_cmd_quiet("brew", &["list", "zsh-autosuggestions"]);
    if has_auto {
        println!("  ✓ zsh-autosuggestions 이미 설치됨");
    } else {
        print!("  ⏳ zsh-autosuggestions 설치 중... ");
        let ok = Command::new("brew")
            .args(["install", "zsh-autosuggestions"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        println!("{}", if ok { "✓" } else { "✗ 실패" });
    }

    // zsh-syntax-highlighting
    let (has_syn, _) = common::run_cmd_quiet("brew", &["list", "zsh-syntax-highlighting"]);
    if has_syn {
        println!("  ✓ zsh-syntax-highlighting 이미 설치됨");
    } else {
        print!("  ⏳ zsh-syntax-highlighting 설치 중... ");
        let ok = Command::new("brew")
            .args(["install", "zsh-syntax-highlighting"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        println!("{}", if ok { "✓" } else { "✗ 실패" });
    }

    println!("\n[workspace] 셸 환경 설정 완료");
}

pub fn bootstrap() {
    println!("=== Mac 작업 환경 부트스트랩 ===\n");

    println!("--- [1/3] tmux + TPM ---");
    install_tmux();

    println!("\n--- [2/3] CLI 도구 ---");
    install_tools();

    println!("\n--- [3/3] 셸 환경 ---");
    setup_shell();

    println!("\n=== 작업 환경 부트스트랩 완료 ===");
}
