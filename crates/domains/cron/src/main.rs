use clap::{Parser, Subcommand};
use mac_host_core::cron;

#[derive(Parser)]
#[command(name = "mac-domain-cron")]
#[command(about = "LaunchAgents 스케줄 작업 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 요약
    Status,
    /// 전체 목록
    List,
    /// 상세 정보
    Info { label: String },
    /// 로드 (시작)
    Load { label: String },
    /// 언로드 (정지)
    Unload { label: String },
    /// 재시작
    Restart { label: String },
    /// 로그 확인
    Logs { label: String },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cron::status(),
        Commands::List => cron::list(),
        Commands::Info { label } => cron::info(&label),
        Commands::Load { label } => cron::load(&label),
        Commands::Unload { label } => cron::unload(&label),
        Commands::Restart { label } => cron::restart(&label),
        Commands::Logs { label } => cron::logs(&label),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    let agents = cron::get_agents();
    let total = agents.len();
    let mine = agents.iter().filter(|a| a.is_mine).count();
    let running = agents.iter().filter(|a| a.running).count();
    let loaded = agents.iter().filter(|a| a.loaded).count();

    let rows: Vec<serde_json::Value> = agents.iter().map(|a| {
        let state = if a.running { "●" } else if a.loaded { "○" } else { "-" };
        serde_json::json!([
            state.to_string(),
            a.label.clone(),
            a.schedule.clone(),
            if a.is_mine { "mine" } else { "system" }.to_string(),
        ])
    }).collect();

    let spec = serde_json::json!({
        "tab": { "label": "Cron", "icon": "⏰" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "LaunchAgents", "value": format!("{}", total), "status": "ok" },
                    { "key": "mac-app-init 관리", "value": format!("{}", mine), "status": "ok" },
                    { "key": "로드됨", "value": format!("{}", loaded), "status": "ok" },
                    { "key": "실행 중", "value": format!("{}", running), "status": "ok" }
                ]
            },
            {
                "kind": "table",
                "title": "Agents",
                "headers": ["", "LABEL", "SCHEDULE", "OWNER"],
                "rows": rows
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status", "command": "status", "key": "s" },
                    { "label": "List", "command": "list", "key": "l" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
