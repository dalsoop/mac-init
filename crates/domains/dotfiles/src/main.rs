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
    /// TUI v2 스펙 (JSON)
    TuiSpec,
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
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let configs = dotfiles::scan_configs();
    let rows: Vec<serde_json::Value> = configs.iter().map(|c| {
        serde_json::json!([
            c.category.to_string(),
            c.name.clone(),
            c.path.display().to_string(),
        ])
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label_ko": "설정파일", "label": "Dotfiles", "icon": "📄" },
        "group": "system",        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "발견된 설정 파일", "value": format!("{} 개", configs.len()), "status": "ok" }
                ]
            },
            {
                "kind": "table",
                "title": "설정 파일",
                "headers": ["CATEGORY", "NAME", "PATH"],
                "rows": rows
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label_ko": "설정파일", "label": "List (목록 갱신)", "command": "list", "key": "l" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
