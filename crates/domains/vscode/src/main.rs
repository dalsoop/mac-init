use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-vscode")]
#[command(about = "VS Code 설치, 확장, 설정 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 확인
    Status,
    /// VS Code 설치
    Install,
    /// 설치된 확장 목록
    ExtList,
    /// 확장 설치 (publisher.extension 형식)
    ExtInstall { id: String },
    /// 확장 제거
    ExtRemove { id: String },
    /// 확장 목록 export (~/.mac-app-init/vscode-extensions.txt)
    ExtExport,
    /// export 파일에서 확장 일괄 설치
    ExtImport,
    /// VS Code 설정 파일 경로 열기
    SettingsPath,
    /// 파일을 VS Code로 열기
    Open { path: String },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

use mac_common::{cmd, paths, tui_spec::{self, TuiSpec}};

fn vscode_app() -> PathBuf {
    PathBuf::from("/Applications/Visual Studio Code.app")
}

fn settings_path() -> PathBuf {
    PathBuf::from(paths::home()).join("Library/Application Support/Code/User/settings.json")
}

fn extensions_export_path() -> PathBuf {
    PathBuf::from(paths::home()).join(".mac-app-init/vscode-extensions.txt")
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Install => cmd_install(),
        Commands::ExtList => cmd_ext_list(),
        Commands::ExtInstall { id } => cmd_ext_install(&id),
        Commands::ExtRemove { id } => cmd_ext_remove(&id),
        Commands::ExtExport => cmd_ext_export(),
        Commands::ExtImport => cmd_ext_import(),
        Commands::SettingsPath => cmd_settings_path(),
        Commands::Open { path } => cmd_open(&path),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let app_installed = vscode_app().exists();
    let code_cli = cmd::ok("which", &["code"]);
    let settings_exists = settings_path().exists();

    let usage_active = code_cli;
    let usage_summary = if code_cli { "VS Code 사용 가능".to_string() }
        else if app_installed { "앱 설치됨 (CLI 미설정)".to_string() }
        else { "미설치".to_string() };

    TuiSpec::new("vscode")
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("Visual Studio Code.app",
                if app_installed { "✓ 설치됨" } else { "✗ 미설치" },
                if app_installed { "ok" } else { "error" }),
            tui_spec::kv_item("code CLI (PATH)",
                if code_cli { "✓ 사용 가능" } else { "✗ 미설치" },
                if code_cli { "ok" } else { "warn" }),
            tui_spec::kv_item("settings.json",
                if settings_exists { "✓ 존재" } else { "✗ 없음" },
                if settings_exists { "ok" } else { "warn" }),
        ])
        .buttons()
        .print();
}

fn cmd_status() {
    println!("=== VS Code 상태 ===\n");

    let app_installed = vscode_app().exists();
    println!("[VS Code.app] {}", if app_installed { "✓ 설치됨" } else { "✗ 미설치" });

    let code_cli = cmd::ok("which", &["code"]);
    if code_cli {
        let ver = cmd::stdout("code", &["--version"]);
        let first_line = ver.lines().next().unwrap_or("");
        println!("[code CLI] ✓ {}", first_line);
    } else {
        println!("[code CLI] ✗ 미설치");
        if app_installed {
            println!("  VS Code에서 Shell Command 설치:");
            println!("  Cmd+Shift+P → 'Shell Command: Install code command in PATH'");
        }
    }

    if !app_installed {
        println!("\n  → mai run vscode install");
    }

    // Settings
    let sp = settings_path();
    println!("\n[설정 파일]");
    if sp.exists() {
        let metadata = fs::metadata(&sp).ok();
        let size = metadata.map(|m| m.len()).unwrap_or(0);
        println!("  ✓ {} ({} bytes)", sp.display(), size);
    } else {
        println!("  ✗ {}", sp.display());
    }

    // Extensions
    if code_cli {
        let exts = cmd::stdout("code", &["--list-extensions"]);
        let count = exts.lines().count();
        println!("\n[확장 프로그램] {} 개", count);
    }
}

fn cmd_install() {
    if vscode_app().exists() {
        println!("✓ VS Code 이미 설치됨");
    } else {
        println!("VS Code 설치 중...");
        let ok = Command::new("brew").args(["install", "--cask", "visual-studio-code"]).status()
            .map(|s| s.success()).unwrap_or(false);
        if ok {
            println!("✓ VS Code 설치 완료");
        } else {
            println!("✗ 설치 실패");
            return;
        }
    }

    // Install code CLI if not available
    if !cmd::ok("which", &["code"]) {
        println!("\ncode CLI 설정 중...");
        let cli_src = "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code";
        let cli_dest = format!("{}/.local/bin/code", paths::home());
        fs::create_dir_all(format!("{}/.local/bin", paths::home())).ok();
        if std::path::Path::new(cli_src).exists() {
            let ok = Command::new("ln").args(["-sf", cli_src, &cli_dest]).status()
                .map(|s| s.success()).unwrap_or(false);
            if ok {
                println!("  ✓ {} 심볼릭 링크 생성", cli_dest);
                println!("  (~/.local/bin이 PATH에 있어야 함)");
            }
        }
    }
}

fn cmd_ext_list() {
    if !cmd::ok("which", &["code"]) {
        println!("✗ code CLI가 없습니다. mai run vscode install");
        return;
    }
    let exts = cmd::stdout("code", &["--list-extensions", "--show-versions"]);
    if exts.is_empty() {
        println!("설치된 확장이 없습니다.");
    } else {
        println!("=== VS Code 확장 ({}) ===\n", exts.lines().count());
        for line in exts.lines() {
            println!("  {}", line);
        }
    }
}

fn cmd_ext_install(id: &str) {
    if !cmd::ok("which", &["code"]) {
        println!("✗ code CLI가 없습니다. mai run vscode install");
        return;
    }
    println!("Installing {}...", id);
    let status = Command::new("code").args(["--install-extension", id]).status();
    match status {
        Ok(s) if s.success() => println!("✓ {} 설치 완료", id),
        _ => println!("✗ 설치 실패"),
    }
}

fn cmd_ext_remove(id: &str) {
    let status = Command::new("code").args(["--uninstall-extension", id]).status();
    match status {
        Ok(s) if s.success() => println!("✓ {} 제거 완료", id),
        _ => println!("✗ 제거 실패"),
    }
}

fn cmd_ext_export() {
    if !cmd::ok("which", &["code"]) {
        println!("✗ code CLI가 없습니다.");
        return;
    }
    let exts = cmd::stdout("code", &["--list-extensions"]);
    let path = extensions_export_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    match fs::write(&path, &exts) {
        Ok(_) => {
            let count = exts.lines().count();
            println!("✓ {} 개 확장 저장: {}", count, path.display());
        }
        Err(e) => println!("✗ 저장 실패: {}", e),
    }
}

fn cmd_ext_import() {
    let path = extensions_export_path();
    if !path.exists() {
        println!("✗ export 파일이 없습니다: {}", path.display());
        println!("  먼저: mai run vscode ext-export");
        return;
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let ids: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    println!("=== {} 개 확장 설치 ===\n", ids.len());
    for id in ids {
        print!("  {} ... ", id);
        let ok = Command::new("code").args(["--install-extension", id]).output()
            .map(|o| o.status.success()).unwrap_or(false);
        println!("{}", if ok { "✓" } else { "✗" });
    }
}

fn cmd_settings_path() {
    let p = settings_path();
    println!("{}", p.display());
    if p.exists() {
        println!("\n  열기: code \"{}\"", p.display());
    }
}

fn cmd_open(path: &str) {
    let status = Command::new("code").args([path]).status();
    match status {
        Ok(s) if s.success() => {}
        _ => {
            let _ = Command::new("open").args(["-a", "Visual Studio Code", path]).status();
        }
    }
}
