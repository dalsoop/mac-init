use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mac-dev-ssl-up", about = "up 도메인")]
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
        Commands::Status => println!("up: ok"),
    }
    Ok(())
}
