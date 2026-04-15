use clap::{Parser, Subcommand};
use mac_host_core::cron;

#[derive(Parser)]
#[command(name = "mac-domain-cron")]
#[command(about = "스케줄 관리 — 내 jobs (schedule.json) + 시스템 LaunchAgents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // --- my jobs (schedule.json) ---
    /// 내 스케줄 jobs 목록
    Jobs,
    /// job 추가 (non-interactive)
    Add {
        /// job 이름
        name: String,
        /// 실행할 쉘 명령
        command: String,
        /// cron 표현식 (예: "*/5 * * * *")
        #[arg(long)]
        cron: Option<String>,
        /// 인터벌 초 (예: 300)
        #[arg(long)]
        interval: Option<u64>,
    },
    /// job 삭제
    Remove { name: String },
    /// job 활성/비활성 토글
    Toggle { name: String },

    // --- scheduler LaunchAgent ---
    /// 스케줄러 LaunchAgent 설치 (매분 tick)
    SetupScheduler,
    /// 스케줄러 LaunchAgent 제거
    RemoveScheduler,

    // --- system LaunchAgents ---
    /// 전체 상태 요약 (내 jobs + 시스템 agents)
    Status,
    /// 시스템 LaunchAgents 목록
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
        Commands::Jobs => cmd_jobs(),
        Commands::Add { name, command, cron: c, interval } => {
            match cron::add_job(&name, &command, c, interval) {
                Ok(msg) => println!("✓ {}", msg),
                Err(e) => eprintln!("✗ {}", e),
            }
        }
        Commands::Remove { name } => match cron::remove_job(&name) {
            Ok(msg) => println!("✓ {}", msg),
            Err(e) => eprintln!("✗ {}", e),
        },
        Commands::Toggle { name } => match cron::toggle_job(&name) {
            Ok((n, en)) => println!("'{}' {}", n, if en { "✓ 활성화" } else { "✗ 비활성화" }),
            Err(e) => eprintln!("✗ {}", e),
        },
        Commands::SetupScheduler => match cron::install_scheduler() {
            Ok(m) => println!("{}", m),
            Err(e) => eprintln!("✗ {}", e),
        },
        Commands::RemoveScheduler => match cron::remove_scheduler() {
            Ok(m) => println!("{}", m),
            Err(e) => eprintln!("✗ {}", e),
        },
        Commands::Status => cmd_status(),
        Commands::List => cron::list(),
        Commands::Info { label } => cron::info(&label),
        Commands::Load { label } => cron::load(&label),
        Commands::Unload { label } => cron::unload(&label),
        Commands::Restart { label } => cron::restart(&label),
        Commands::Logs { label } => cron::logs(&label),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn cmd_jobs() {
    let s = cron::load_schedule();
    if s.jobs.is_empty() {
        println!("등록된 작업이 없습니다.");
        println!("  mac run cron add <name> <command> --cron \"0 9 * * *\"");
        return;
    }
    println!("{:<20} {:<8} {:<25} {}", "NAME", "STATUS", "SCHEDULE", "COMMAND");
    println!("{}", "─".repeat(80));
    for j in &s.jobs {
        let st = if j.enabled { "✓" } else { "✗" };
        let sc = match j.schedule.stype.as_str() {
            "cron" => j.schedule.cron.clone().unwrap_or_default(),
            "interval" => format!("every {}s", j.schedule.interval_seconds.unwrap_or(0)),
            _ => "?".into(),
        };
        println!("{:<20} {:<8} {:<25} {}", j.name, st, sc, j.command);
    }
}

fn cmd_status() {
    let sched = cron::load_schedule();
    let scheduler_on = cron::scheduler_installed();
    let agents = cron::get_agents();
    let mine = agents.iter().filter(|a| a.is_mine).count();
    let running = agents.iter().filter(|a| a.running).count();

    println!("=== Cron 상태 ===\n");
    println!("스케줄러 LaunchAgent: {}", if scheduler_on { "✓ 설치됨" } else { "✗ 미설치 (mac run cron setup-scheduler)" });
    println!("내 스케줄 jobs:       {}개 (활성 {}개)",
        sched.jobs.len(),
        sched.jobs.iter().filter(|j| j.enabled).count(),
    );
    println!("시스템 LaunchAgents:  {}개 (내 서비스 {}, 실행 중 {})", agents.len(), mine, running);
}

fn print_tui_spec() {
    let sched = cron::load_schedule();
    let scheduler_on = cron::scheduler_installed();
    let agents = cron::get_agents();

    let job_rows: Vec<serde_json::Value> = sched.jobs.iter().map(|j| {
        let sc = match j.schedule.stype.as_str() {
            "cron" => j.schedule.cron.clone().unwrap_or_default(),
            "interval" => format!("every {}s", j.schedule.interval_seconds.unwrap_or(0)),
            _ => "?".into(),
        };
        serde_json::json!([
            if j.enabled { "✓" } else { "✗" },
            j.name,
            sc,
            j.command,
        ])
    }).collect();

    let agent_rows: Vec<serde_json::Value> = agents.iter().map(|a| {
        let state = if a.running { "●" } else if a.loaded { "○" } else { "-" };
        serde_json::json!([
            state,
            a.label.clone(),
            a.schedule.clone(),
            if a.is_mine { "mine" } else { "system" },
        ])
    }).collect();

    let total_agents = agents.len();
    let mine_agents = agents.iter().filter(|a| a.is_mine).count();
    let active_jobs = sched.jobs.iter().filter(|j| j.enabled).count();

    let spec = serde_json::json!({
        "tab": { "label": "Cron", "icon": "⏰" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    {
                        "key": "스케줄러 LaunchAgent",
                        "value": if scheduler_on { "✓ 설치됨" } else { "✗ 미설치" },
                        "status": if scheduler_on { "ok" } else { "error" }
                    },
                    {
                        "key": "내 jobs (활성/총)",
                        "value": format!("{} / {}", active_jobs, sched.jobs.len()),
                        "status": if sched.jobs.is_empty() { "warn" } else { "ok" }
                    },
                    {
                        "key": "시스템 LaunchAgents",
                        "value": format!("{} (내 서비스 {})", total_agents, mine_agents),
                        "status": "ok"
                    }
                ]
            },
            {
                "kind": "table",
                "title": "내 스케줄 jobs (schedule.json)",
                "headers": ["", "NAME", "SCHEDULE", "COMMAND"],
                "rows": job_rows
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Jobs (내 jobs)", "command": "jobs", "key": "j" },
                    { "label": "Status", "command": "status", "key": "s" },
                    { "label": "List (시스템 agents)", "command": "list", "key": "l" },
                    { "label": "Setup Scheduler", "command": "setup-scheduler", "key": "u" },
                    { "label": "Remove Scheduler", "command": "remove-scheduler", "key": "x" }
                ]
            },
            {
                "kind": "table",
                "title": "시스템 LaunchAgents",
                "headers": ["", "LABEL", "SCHEDULE", "OWNER"],
                "rows": agent_rows
            },
            {
                "kind": "text",
                "title": "Job 추가 / 삭제 — 터미널",
                "content": "  추가: mac run cron add <name> \"<command>\" --cron \"*/5 * * * *\"\n        mac run cron add <name> \"<command>\" --interval 300\n  삭제: mac run cron remove <name>\n  토글: mac run cron toggle <name>"
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
