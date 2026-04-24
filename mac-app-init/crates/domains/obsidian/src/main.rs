use clap::{Parser, Subcommand};
use mac_common::tui_spec::{self, TuiSpec};

#[derive(Parser)]
#[command(name = "mac-domain-obsidian")]
#[command(about = "Obsidian 볼트 동기화 + 플러그인 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 상태 확인
    Status,
    /// TUI 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn cmd_status() {
    println!("=== Obsidian 관리 ===\n");
    println!("TODO: 상태 출력 구현");
}

fn print_tui_spec() {
    TuiSpec::new("obsidian")
        .usage(false, "미설정")
        .kv("상태", vec![tui_spec::kv_item("상태", "TODO", "warn")])
        .buttons()
        .text("안내", "TODO: 도메인 설명 작성")
        .print();
}
