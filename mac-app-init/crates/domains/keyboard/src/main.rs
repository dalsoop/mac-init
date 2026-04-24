use clap::{Parser, Subcommand};
use mac_host_core::keyboard;

#[derive(Parser)]
#[command(name = "mac-domain-keyboard")]
#[command(about = "Caps Lock → F18 한영 전환 (hidutil)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 키보드 매핑 상태 확인
    Status,
    /// Caps Lock → F18 매핑 설정 + LaunchAgent 등록
    Setup,
    /// 매핑 제거 + LaunchAgent 삭제
    Remove,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => keyboard::print_status(),
        Commands::Setup => keyboard::print_setup(),
        Commands::Remove => keyboard::print_remove(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    use mac_common::tui_spec::{self, TuiSpec};

    let s = keyboard::get_status();
    let usage_active = s.mapping_active;
    let usage_summary = if s.mapping_active {
        "F18 적용됨".to_string()
    } else {
        "미적용".to_string()
    };

    TuiSpec::new("keyboard")
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("Caps Lock → F18",
                if s.mapping_active { "✓ 적용됨" } else { "✗ 미적용" },
                if s.mapping_active { "ok" } else { "error" }),
            tui_spec::kv_item("부팅 시 자동 적용",
                if s.launch_agent_exists { "✓ 등록됨" } else { "✗ 미등록" },
                if s.launch_agent_exists { "ok" } else { "error" }),
            tui_spec::kv_item("Karabiner",
                if s.karabiner_installed { "⚠ 설치됨 (제거 권장)" } else { "✓ 미설치" },
                if s.karabiner_installed { "warn" } else { "ok" }),
        ])
        .buttons()
        .text("안내", "시스템 설정 → 키보드 → 키보드 단축키 → 입력 소스\n'이전 입력 소스 선택' = F18 로 설정 필요")
        .print();
}
