use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mac-dev-ssl-proxy", about = "proxy 도메인")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 상태 출력
    Status,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => println!("proxy: ok"),
    }
    Ok(())
}
