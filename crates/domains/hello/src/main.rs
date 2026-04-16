use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rustai-hello", about = "hello 도메인")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 인사하기
    Greet {
        #[arg(default_value = "world")]
        target: String,
    },
    /// 상태
    Status,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Greet { target } => println!("hello, {target}"),
        Commands::Status => println!("hello: ok"),
    }
    Ok(())
}
