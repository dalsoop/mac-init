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
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => worktree::status(),
        Commands::Add { project, btype, name } => worktree::add(&project, &btype, &name),
        Commands::Remove { project, btype, name } => worktree::remove(&project, &btype, &name),
        Commands::Clean => worktree::clean(),
    }
}
