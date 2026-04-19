use clap::{Parser, Subcommand};
use mac_host_core::worktree;

#[derive(Parser)]
#[command(name = "mac-domain-worktree")]
#[command(about = "Git worktree 브랜치별 폴더 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// worktree 상태
    Status,
    /// worktree 생성
    Add { project: String, #[arg(name = "type")] btype: String, name: String },
    /// worktree 제거
    Remove { project: String, #[arg(name = "type")] btype: String, name: String },
    /// 머지 완료 + stale 자동 정리
    Clean,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => worktree::status(),
        Commands::Add { project, btype, name } => worktree::add(&project, &btype, &name),
        Commands::Remove { project, btype, name } => worktree::remove(&project, &btype, &name),
        Commands::Clean => worktree::clean(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let spec = serde_json::json!({
        "tab": { "label_ko": "워크트리", "label": "Worktree", "icon": "🌿" },
        "group": "auto",        "sections": [
            {
                "kind": "text",
                "title": "설명",
                "content": "Git worktree 기반 브랜치별 폴더 관리.\nStatus로 현재 worktree 목록, Clean으로 머지/stale 정리."
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label_ko": "워크트리", "label": "Status (worktree 상태)", "command": "status", "key": "s" },
                    { "label_ko": "워크트리", "label": "Clean (정리)", "command": "clean", "key": "c" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
