use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rustai-ping", about = "ping 도메인 (hello에 의존)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 한 번 핑
    Once,
    /// 반복 핑
    Loop {
        #[arg(long, default_value_t = 3)]
        count: u32,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Once => println!("pong"),
        Commands::Loop { count } => {
            for i in 0..count {
                println!("pong {}", i + 1);
            }
        }
    }
    Ok(())
}
