use clap::{Parser, Subcommand};
use mac_host_core::files;

#[derive(Parser)]
#[command(name = "mac-domain-files")]
#[command(about = "파일 자동 분류, lint")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 파일 관리 상태
    Status,
    /// Downloads 파일 자동 분류
    Organize,
    /// 임시 폴더 정리
    CleanupTemp,
    /// 자동 정리 활성화
    SetupAuto,
    /// 자동 정리 비활성화
    DisableAuto,
    /// 파일 lint
    Lint,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => files::status(),
        Commands::Organize => files::organize(),
        Commands::CleanupTemp => files::cleanup_temp(),
        Commands::SetupAuto => files::setup_auto(),
        Commands::DisableAuto => files::disable_auto(),
        Commands::Lint => files::lint(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    use mac_common::{
        paths,
        tui_spec::{self, TuiSpec},
    };
    use std::path::PathBuf;

    let home = paths::home();
    let dl = PathBuf::from(&home).join("Downloads");
    let dl_count = std::fs::read_dir(&dl).map(|it| it.count()).unwrap_or(0);
    let tmp = PathBuf::from(&home).join("Documents/WORK/임시");
    let tmp_count = std::fs::read_dir(&tmp).map(|it| it.count()).unwrap_or(0);
    let auto_plist =
        PathBuf::from(&home).join("Library/LaunchAgents/com.mac-host.file-organizer.plist");
    let auto_on = auto_plist.exists();

    let usage_active = auto_on;
    let usage_summary = if auto_on {
        "자동정리 켜짐".to_string()
    } else {
        "꺼짐".to_string()
    };

    TuiSpec::new("files")
        .refresh(30)
        .usage(usage_active, &usage_summary)
        .kv(
            "상태",
            vec![
                tui_spec::kv_item(
                    "Downloads",
                    &format!("{}개 파일", dl_count),
                    if dl_count > 20 { "warn" } else { "ok" },
                ),
                tui_spec::kv_item(
                    "임시 폴더",
                    &format!("{}개 파일", tmp_count),
                    if tmp_count > 10 { "warn" } else { "ok" },
                ),
                tui_spec::kv_item(
                    "자동 정리",
                    if auto_on { "✓ 켜짐" } else { "꺼짐" },
                    if auto_on { "ok" } else { "warn" },
                ),
            ],
        )
        .buttons()
        .buttons_custom(
            "토글",
            vec![serde_json::json!({
                "label": if auto_on { "자동정리 OFF" } else { "자동정리 ON" },
                "command": if auto_on { "disable-auto" } else { "setup-auto" },
                "key": "a"
            })],
        )
        .print();
}
