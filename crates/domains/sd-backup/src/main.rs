use clap::{Parser, Subcommand};
use mac_host_core::files;

#[derive(Parser)]
#[command(name = "mac-domain-sd-backup")]
#[command(about = "SD 카드 미디어 자동 백업")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// SD 백업 상태
    Status,
    /// SD 백업 실행
    Run,
    /// SD 백업 자동화 활성화 (LaunchAgent)
    Enable,
    /// SD 백업 자동화 비활성화
    Disable,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => files::sd_status(),
        Commands::Run => files::sd_run(),
        Commands::Enable => files::sd_enable(),
        Commands::Disable => files::sd_disable(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let spec = serde_json::json!({
        "tab": { "label_ko": "SD 미디어 백업", "label": "SD Backup", "icon": "📸" },
        "group": "auto",
        "sections": [
            {
                "kind": "text",
                "title": "설명",
                "content": "SD 카드를 꽂으면 자동으로 미디어 파일을 백업합니다.\nEnable 로 LaunchAgent 등록, Run 으로 수동 실행."
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status (백업 상태)", "command": "status", "key": "s" },
                    { "label": "Run (수동 백업)", "command": "run", "key": "r" },
                    { "label": "Enable (자동화 켜기)", "command": "enable", "key": "e" },
                    { "label": "Disable (자동화 끄기)", "command": "disable", "key": "d" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
