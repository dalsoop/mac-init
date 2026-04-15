use clap::{Parser, Subcommand};
use mac_host_core::files;

#[derive(Parser)]
#[command(name = "mac-domain-files")]
#[command(about = "파일 자동 분류, SD 백업, lint")]
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
    /// SD 백업 상태
    SdStatus,
    /// SD 백업 활성화
    SdEnable,
    /// SD 백업 비활성화
    SdDisable,
    /// SD 백업 실행
    SdRun,
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
        Commands::SdStatus => files::sd_status(),
        Commands::SdEnable => files::sd_enable(),
        Commands::SdDisable => files::sd_disable(),
        Commands::SdRun => files::sd_run(),
        Commands::Lint => files::lint(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let spec = serde_json::json!({
        "tab": { "label": "Files", "icon": "📁" },
        "sections": [
            {
                "kind": "text",
                "title": "설명",
                "content": "Downloads 자동 분류, 임시 폴더 정리, SD 백업, lint 유틸리티.\n자세한 상태는 아래 Status / SD Status 버튼으로 확인하세요."
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status (파일 관리 상태)", "command": "status", "key": "s" },
                    { "label": "Organize (Downloads 자동 분류)", "command": "organize", "key": "o" },
                    { "label": "Cleanup Temp (임시 정리)", "command": "cleanup-temp", "key": "c" },
                    { "label": "Setup Auto (자동화 활성화)", "command": "setup-auto", "key": "a" },
                    { "label": "SD Status (SD 백업 상태)", "command": "sd-status", "key": "d" },
                    { "label": "Lint (파일 lint)", "command": "lint", "key": "l" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
