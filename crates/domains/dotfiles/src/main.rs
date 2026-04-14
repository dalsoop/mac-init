use clap::{Parser, Subcommand};
use mac_host_core::dotfiles;

#[derive(Parser)]
#[command(name = "mac-domain-dotfiles")]
#[command(about = "dotfiles 설정 파일 스캔/읽기")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 설정 파일 목록
    List,
    /// 파일 내용 보기
    Read { path: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            for c in dotfiles::scan_configs() {
                println!("{:<12} {:<20} {}", c.category, c.name, c.path.display());
            }
        }
        Commands::Read { path } => {
            let p = std::path::PathBuf::from(&path);
            match dotfiles::read_config(&p) {
                Some(content) => print!("{}", content),
                None => eprintln!("파일을 읽을 수 없습니다: {}", path),
            }
        }
    }
}
