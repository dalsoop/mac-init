use clap::{Parser, Subcommand};
use mac_host_core::defaults;

#[derive(Parser)]
#[command(name = "mac-domain-defaults")]
#[command(about = "macOS 시스템 설정 (defaults) 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 도메인 목록
    List,
    /// 도메인 내 키/값 보기
    Read { domain: String },
    /// 값 쓰기
    Write { domain: String, key: String, #[arg(long)] r#type: String, value: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            for d in defaults::list_domains() {
                println!("{}", d);
            }
        }
        Commands::Read { domain } => {
            for e in defaults::read_domain(&domain) {
                println!("{:<30} {:<10} {}", e.key, e.value_type, e.value);
            }
        }
        Commands::Write { domain, key, r#type, value } => {
            match defaults::write_default(&domain, &key, &r#type, &value) {
                Ok(out) => print!("{}", out),
                Err(e) => eprintln!("{}", e),
            }
        }
    }
}
