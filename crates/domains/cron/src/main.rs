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

    /// 전체 상태 요약 (내 jobs)
    Status,

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
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn cmd_jobs() {
    let s = cron::load_schedule();
    if s.jobs.is_empty() {
        println!("등록된 작업이 없습니다.");
        println!("  mai run cron add <name> <command> --cron \"0 9 * * *\"");
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

    println!("=== Cron 상태 (mac-app-init 스케줄) ===\n");
    println!("스케줄러 LaunchAgent: {}", if scheduler_on { "✓ 설치됨" } else { "✗ 미설치 (mai run cron setup-scheduler)" });
    println!("스케줄 jobs: {}개 (활성 {}개)",
        sched.jobs.len(),
        sched.jobs.iter().filter(|j| j.enabled).count(),
    );
    println!("\n시스템 LaunchAgent 조회는: mai run bootstrap agent list");
}

fn print_tui_spec() {
    use mac_common::tui_spec::{self, TuiSpec};

    let sched = cron::load_schedule();
    let scheduler_on = cron::scheduler_installed();

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

    let active_jobs = sched.jobs.iter().filter(|j| j.enabled).count();

    let usage_active = active_jobs > 0;
    let usage_summary = format!("{}개 활성", active_jobs);

    TuiSpec::new("cron")
        .refresh(30)
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("스케줄러",
                if scheduler_on { "✓ 설치됨 (매분 tick)" } else { "✗ 미설치" },
                if scheduler_on { "ok" } else { "error" }),
            tui_spec::kv_item("스케줄 jobs",
                &format!("{} / {} 활성", active_jobs, sched.jobs.len()),
                if sched.jobs.is_empty() { "warn" } else { "ok" }),
        ])
        .table("스케줄", vec!["", "NAME", "SCHEDULE", "COMMAND"], job_rows)
        .buttons()
        .text("안내", "  mai run cron add <name> \"<command>\" --cron \"*/5 * * * *\"\n  mai run cron add <name> \"<command>\" --interval 300\n  mai run cron remove/toggle <name>\n\n  시스템 LaunchAgent 조회: mai run bootstrap agent list")
        .print();
}
