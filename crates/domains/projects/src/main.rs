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
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => projects::status(),
        Commands::Sync => projects::sync_ncl(),
    }
}
