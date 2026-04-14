use clap::{Parser, Subcommand};
use mac_host_core::keyboard;

#[derive(Parser)]
#[command(name = "mac-domain-keyboard")]
#[command(about = "Caps Lock → F18 한영 전환 (hidutil)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 키보드 매핑 상태 확인
    Status,
    /// Caps Lock → F18 매핑 설정 + LaunchAgent 등록
    Setup,
    /// 매핑 제거 + LaunchAgent 삭제
    Remove,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => keyboard::print_status(),
        Commands::Setup => keyboard::print_setup(),
        Commands::Remove => keyboard::print_remove(),
    }
}
