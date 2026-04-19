use clap::{Parser, Subcommand};
use mac_host_core::files;

#[derive(Parser)]
#[command(name = "mac-domain-files")]
#[command(about = "파일 자동 분류, lint")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 파일 관리 상태
    Status,
    /// Downloads 파일 자동 분류
    Organize,
    /// 임시 폴더 정리
    CleanupTemp,
    /// 자동 정리 활성화
    SetupAuto,
    /// 자동 정리 비활성화
    DisableAuto,
    /// 파일 lint
    Lint,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => files::status(),
        Commands::Organize => files::organize(),
        Commands::CleanupTemp => files::cleanup_temp(),
        Commands::SetupAuto => files::setup_auto(),
        Commands::DisableAuto => files::disable_auto(),
        Commands::Lint => files::lint(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let spec = serde_json::json!({
        "tab": { "label_ko": "파일정리", "label": "Files", "icon": "📁" },
        "group": "auto",        "sections": [
            {
                "kind": "text",
                "title": "설명",
                "content": "Downloads 자동 분류, 임시 폴더 정리, lint 유틸리티.\nSD 백업은 별도 도메인 (SD 미디어 백업) 참조."
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label_ko": "파일정리", "label": "Status (파일 관리 상태)", "command": "status", "key": "s" },
                    { "label_ko": "파일정리", "label": "Organize (Downloads 자동 분류)", "command": "organize", "key": "o" },
                    { "label_ko": "파일정리", "label": "Cleanup Temp (임시 정리)", "command": "cleanup-temp", "key": "c" },
                    { "label_ko": "파일정리", "label": "Setup Auto (자동화 활성화)", "command": "setup-auto", "key": "a" },
                    { "label_ko": "파일정리", "label": "Lint (파일 lint)", "command": "lint", "key": "l" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
