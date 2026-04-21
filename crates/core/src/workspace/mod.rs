use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::common;

fn command_version(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn file_mark(path: &str) -> &'static str {
    if Path::new(path).exists() { "✓" } else { "✗" }
}

fn run_step(cmd: &str, args: &[&str]) -> bool {
    println!("  → {} {}", cmd, args.join(" "));
    Command::new(cmd)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn load_env_map(path: &Path) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                pairs.push((k.trim().to_string(), v.trim().trim_matches('"').to_string()));
            }
        }
    }
    pairs
}

fn env_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
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
        ("lazydocker", "Docker TUI"),
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
        "lazydocker",
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
pub fn ai_status() {
    let home = home();
    let claude_auth = format!("{home}/.claude.json");
    let codex_auth = format!("{home}/.codex/auth.json");
    let opencode_auth = format!("{home}/.local/share/opencode/auth.json");
    let opencode_config = format!("{home}/.config/opencode/opencode.json");
    let omx_agents = format!("{home}/.codex/AGENTS.md");
    let env_path = common::env_file();
    let env_pairs = load_env_map(&env_path);

    println!("=== 로컬 AI 작업 환경 상태 ===\n");

    println!("[CLI]");
    for (cmd, args, name) in [
        ("claude", vec!["--version"], "Claude Code"),
        ("codex", vec!["--version"], "Codex CLI"),
        ("omx", vec!["--version"], "oh-my-codex"),
        ("opencode", vec!["--version"], "OpenCode"),
    ] {
        match command_version(cmd, &args) {
            Some(version) => println!("  ✓ {name} {version}"),
            None => println!("  ✗ {name}"),
        }
    }

    println!("\n[인증 파일]");
    println!("  {} Claude auth  ({claude_auth})", file_mark(&claude_auth));
    println!("  {} Codex auth   ({codex_auth})", file_mark(&codex_auth));
    println!("  {} OpenCode auth ({opencode_auth})", file_mark(&opencode_auth));

    println!("\n[설정]");
    println!("  {} OMX AGENTS   ({omx_agents})", file_mark(&omx_agents));
    println!("  {} OpenCode cfg ({opencode_config})", file_mark(&opencode_config));

    if Path::new(&opencode_config).exists() {
        let content = std::fs::read_to_string(&opencode_config).unwrap_or_default();
        let plugin_ok = content.contains("oh-my-openagent") || content.contains("oh-my-opencode");
        println!("  {} oh-my-openagent 플러그인", if plugin_ok { "✓" } else { "✗" });
    }

    let (ok, auth_list) = common::run_cmd_quiet("opencode", &["auth", "list"]);
    if ok {
        let compact = auth_list.replace('\u{1b}', "");
        if compact.contains("0 credentials") {
            println!("\n[OpenCode auth] 아직 로그인 없음");
        } else {
            println!("\n[OpenCode auth] 설정됨");
        }
    }

    println!("\n[공급자 키]");
    println!("  {} env file ({})", file_mark(&env_path.display().to_string()), env_path.display());
    println!(
        "  {} GOOGLE_API_KEY",
        if env_value(&env_pairs, "GOOGLE_API_KEY").is_some() { "✓" } else { "✗" }
    );
    println!(
        "  {} GEMINI_API_KEY",
        if env_value(&env_pairs, "GEMINI_API_KEY").is_some() { "✓" } else { "✗" }
    );
    println!(
        "  {} GOOGLE_GENERATIVE_AI_API_KEY",
        if env_value(&env_pairs, "GOOGLE_GENERATIVE_AI_API_KEY").is_some() { "✓" } else { "✗" }
    );
    println!(
        "  {} MINIMAX_API_KEY",
        if env_value(&env_pairs, "MINIMAX_API_KEY").is_some() { "✓" } else { "✗" }
    );
}

pub fn ai_setup() {
    println!("=== 로컬 AI 작업 환경 정리 ===\n");

    let omx_exists = command_version("omx", &["--version"]).is_some();
    if omx_exists {
        println!("[ai] omx setup --force 실행 중...");
        let ok = Command::new("omx")
            .args(["setup", "--force"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            println!("[ai] OMX 설정 완료");
        } else {
            eprintln!("[ai] OMX 설정 실패");
        }
    } else {
        eprintln!("[ai] omx 가 설치되어 있지 않습니다.");
    }

    let opencode_config = format!("{}/.config/opencode/opencode.json", home());
    if Path::new(&opencode_config).exists() {
        println!("[ai] OpenCode 설정 파일 확인 완료");
    } else {
        eprintln!("[ai] OpenCode 설정이 없습니다. oh-my-openagent 설치가 먼저 필요합니다.");
    }

    println!("\n[ai] 현재 Mac에는 Claude/Codex 로컬 인증 파일이 있지만, OpenCode auth는 별도 로그인 상태입니다.");
    println!("[ai] 자세한 상태는 `mai workspace ai-status`로 확인하세요.");
}

pub fn ai_reinstall_opencode() {
    let home = home();
    let cwd = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let targets = [
        format!("{home}/.cache/opencode"),
        format!("{home}/.config/opencode"),
        format!("{home}/.local/share/opencode"),
        cwd.join(".opencode").display().to_string(),
    ];

    println!("=== OpenCode 초기화 + 재설치 ===\n");

    println!("[reset] 기존 OpenCode 캐시/설정 삭제");
    for target in targets {
        let path = Path::new(&target);
        if path.exists() {
            if let Err(err) = fs::remove_dir_all(path) {
                eprintln!("[reset] 삭제 실패 ({}): {err}", path.display());
                std::process::exit(1);
            }
            println!("  ✓ {}", path.display());
        } else {
            println!("  - {} (없음)", path.display());
        }
    }

    println!("\n[install] oh-my-openagent 비대화식 재설치");
    let install_ok = run_step(
        "npx",
        &[
            "oh-my-opencode",
            "install",
            "--no-tui",
            "--claude=yes",
            "--openai=yes",
            "--gemini=no",
            "--copilot=no",
            "--opencode-zen=no",
            "--opencode-go=no",
            "--zai-coding-plan=no",
        ],
    );
    if !install_ok {
        eprintln!("[install] oh-my-openagent 설치 실패");
        std::process::exit(1);
    }

    println!("\n[install] OpenCode 업그레이드");
    if !run_step("opencode", &["upgrade"]) {
        eprintln!("[install] opencode upgrade 실패");
        std::process::exit(1);
    }

    println!("\n[plugin] 필수 플러그인 재설치");
    let plugins = [
        "oh-my-openagent",
        "opencode-anthropic-auth",
        "opencode-openai-codex-auth",
    ];
    for plugin in plugins {
        let ok = run_step("opencode", &["plugin", plugin, "-g", "-f"]);
        if !ok {
            eprintln!("[plugin] {plugin} 설치 실패");
            std::process::exit(1);
        }
    }

    println!("\n[verify] 현재 상태 확인");
    let _ = run_step("opencode", &["--version"]);
    let _ = run_step("npx", &["oh-my-opencode", "doctor"]);

    println!("\n[done] OpenCode 재설치 완료");
    println!("[done] 다음 단계: `opencode auth login -p anthropic -m \"Claude Pro/Max\"`");
    println!("[done] 확인: `mai workspace ai-status`");
}

pub fn ai_set_provider_keys(google_api_key: Option<&str>, minimax_api_key: Option<&str>) {
    let env_path = common::env_file();
    let env_dir = env_path.parent().unwrap_or_else(|| Path::new("."));
    common::ensure_dir(env_dir);

    let mut pairs = load_env_map(&env_path);

    let upsert = |pairs: &mut Vec<(String, String)>, key: &str, value: &str| {
        if let Some((_, existing)) = pairs.iter_mut().find(|(k, _)| k == key) {
            *existing = value.to_string();
        } else {
            pairs.push((key.to_string(), value.to_string()));
        }
    };

    if let Some(value) = google_api_key {
        upsert(&mut pairs, "GOOGLE_API_KEY", value);
        upsert(&mut pairs, "GEMINI_API_KEY", value);
        upsert(&mut pairs, "GOOGLE_GENERATIVE_AI_API_KEY", value);
    }
    if let Some(value) = minimax_api_key {
        upsert(&mut pairs, "MINIMAX_API_KEY", value);
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let mut output = String::from("# mai environment\n");
    for (key, value) in pairs {
        output.push_str(&format!("{key}={value}\n"));
    }

    fs::write(&env_path, output).unwrap_or_else(|err| {
        eprintln!("[ai] 환경파일 저장 실패 ({}): {err}", env_path.display());
        std::process::exit(1);
    });

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        if let Err(err) = fs::set_permissions(&env_path, perms) {
            eprintln!("[ai] 환경파일 권한 설정 실패 ({}): {err}", env_path.display());
            std::process::exit(1);
        }
    }

    println!("[ai] 공급자 키 저장 완료");
    if google_api_key.is_some() {
        println!("  ✓ GOOGLE_API_KEY");
        println!("  ✓ GEMINI_API_KEY");
        println!("  ✓ GOOGLE_GENERATIVE_AI_API_KEY");
    }
    if minimax_api_key.is_some() {
        println!("  ✓ MINIMAX_API_KEY");
    }
    println!("  → {}", env_path.display());
}

pub fn ai_start_omx() {
    println!("[ai] oh-my-codex 실행: omx --madmax --high");
    let ok = Command::new("omx")
        .args(["--madmax", "--high"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !ok {
        eprintln!("[ai] omx 실행 실패");
        std::process::exit(1);
    }
}
