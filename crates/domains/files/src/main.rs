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
    }
}
