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
    let s = keyboard::get_status();
    let spec = serde_json::json!({
        "tab": { "label_ko": "키보드", "label": "Keyboard", "icon": "⌨" },
        "group": "system",        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    {
                        "key": "Caps Lock → F18",
                        "value": if s.mapping_active { "✓ 적용됨" } else { "✗ 미적용" },
                        "status": if s.mapping_active { "ok" } else { "error" }
                    },
                    {
                        "key": "부팅 시 자동 적용",
                        "value": if s.launch_agent_exists { "✓ 등록됨" } else { "✗ 미등록" },
                        "status": if s.launch_agent_exists { "ok" } else { "error" }
                    },
                    {
                        "key": "Karabiner",
                        "value": if s.karabiner_installed { "⚠ 설치됨 (제거 권장)" } else { "✓ 미설치" },
                        "status": if s.karabiner_installed { "warn" } else { "ok" }
                    }
                ]
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label_ko": "키보드", "label": "Setup (매핑 + LaunchAgent 등록)", "command": "setup", "key": "s" },
                    { "label_ko": "키보드", "label": "Remove (매핑 + LaunchAgent 삭제)", "command": "remove", "key": "x" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "시스템 설정 → 키보드 → 키보드 단축키 → 입력 소스\n'이전 입력 소스 선택' = F18 로 설정 필요"
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
