use clap::{Parser, Subcommand};
use mac_host_core::brew;

#[derive(Parser)]
#[command(name = "mac-domain-brew")]
#[command(about = "Homebrew 패키지 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 설치된 패키지 목록
    List,
    /// 패키지 설치
    Install { name: String, #[arg(long)] cask: bool },
    /// 패키지 삭제
    Uninstall { name: String, #[arg(long)] cask: bool },
    /// 패키지 업그레이드
    Upgrade { name: String, #[arg(long)] cask: bool },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            let pkgs = brew::list_installed();
            for p in &pkgs {
                let tag = if p.is_cask { "cask" } else { "formula" };
                let flag = if p.outdated { " [outdated]" } else { "" };
                println!("{:<8} {:<30} {}{}", tag, p.name, p.version, flag);
            }
            println!("\n{} packages", pkgs.len());
        }
        Commands::Install { name, cask } => match brew::install(&name, cask) {
            Ok(out) => print!("{}", out),
            Err(e) => eprintln!("{}", e),
        },
        Commands::Uninstall { name, cask } => match brew::uninstall(&name, cask) {
            Ok(out) => print!("{}", out),
            Err(e) => eprintln!("{}", e),
        },
        Commands::Upgrade { name, cask } => match brew::upgrade(&name, cask) {
            Ok(out) => print!("{}", out),
            Err(e) => eprintln!("{}", e),
        },
    }
}
