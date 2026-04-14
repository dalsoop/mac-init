use clap::{Parser, Subcommand};
use mac_host_core::cron;

#[derive(Parser)]
#[command(name = "mac-domain-cron")]
#[command(about = "LaunchAgents 스케줄 작업 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 요약
    Status,
    /// 전체 목록
    List,
    /// 상세 정보
    Info { label: String },
    /// 로드 (시작)
    Load { label: String },
    /// 언로드 (정지)
    Unload { label: String },
    /// 재시작
    Restart { label: String },
    /// 로그 확인
    Logs { label: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cron::status(),
        Commands::List => cron::list(),
        Commands::Info { label } => cron::info(&label),
        Commands::Load { label } => cron::load(&label),
        Commands::Unload { label } => cron::unload(&label),
        Commands::Restart { label } => cron::restart(&label),
        Commands::Logs { label } => cron::logs(&label),
    }
}
