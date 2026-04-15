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
    /// TUI v2 스펙 (JSON)
    TuiSpec,
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
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let domains = defaults::list_domains();
    let spec = serde_json::json!({
        "tab": { "label": "Defaults", "icon": "⚙" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "등록된 도메인", "value": format!("{} 개", domains.len()), "status": "ok" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "defaults 도메인 값을 보려면 `mac run defaults list` 또는 `mac run defaults read <domain>` 사용."
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "List (도메인 목록)", "command": "list", "key": "l" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
