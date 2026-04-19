use clap::{Parser, Subcommand};
use mac_host_core::projects;

#[derive(Parser)]
#[command(name = "mac-domain-projects")]
#[command(about = "프로젝트 스캔/동기화")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 프로젝트 목록
    Status,
    /// NCL 동기화
    Sync,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => projects::status(),
        Commands::Sync => projects::sync_ncl(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let spec = serde_json::json!({
        "tab": { "label_ko": "프로젝트", "label": "Projects", "icon": "📚" },
        "group": "auto",        "sections": [
            {
                "kind": "text",
                "title": "설명",
                "content": "프로젝트 스캔 및 NCL 메타데이터 동기화.\nStatus 버튼으로 프로젝트 목록 확인, Sync로 ncl/ 재생성."
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label_ko": "프로젝트", "label": "Status (프로젝트 목록)", "command": "status", "key": "s" },
                    { "label_ko": "프로젝트", "label": "Sync (NCL 동기화)", "command": "sync", "key": "y" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
