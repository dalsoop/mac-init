use mac_host_tui::app::App;
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io;
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // ── 시작 전 업데이트 체크 (raw mode 진입 전) ──
    if let Some(latest) = check_latest_version() {
        if latest != VERSION {
            println!("⚠ 업데이트 필요: 현재 {} → 최신 {}", VERSION, latest);
            println!();
            print!("업데이트 하시겠습니까? (Y/n) ");
            io::Write::flush(&mut io::stdout())?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let answer = input.trim().to_lowercase();
            if answer.is_empty() || answer == "y" || answer == "yes" {
                println!("\n업데이트 실행 중...");
                let status = Command::new("mai").arg("upgrade").status();
                match status {
                    Ok(s) if s.success() => {
                        println!("✓ 업데이트 완료. TUI를 재시작합니다.\n");
                        // exec()로 자기 교체 — 새 바이너리로 재시작
                        let err = std::os::unix::process::CommandExt::exec(
                            Command::new(std::env::current_exe().unwrap_or_else(|_| "mai-tui".into()))
                                .args(std::env::args().skip(1))
                        );
                        eprintln!("재시작 실패: {}", err);
                        std::process::exit(1);
                    }
                    _ => {
                        eprintln!("✗ 업데이트 실패. 수동: mai upgrade");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("업데이트를 건너뛰면 진행할 수 없습니다.");
                std::process::exit(0);
            }
        }
    }

    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(EnableMouseCapture)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();

    // 사이드바 즉시 표시. spec은 백그라운드에서 전체 프리로드.
    app.load_fast();
    app.preload_all_specs();

    let mut last_refresh = std::time::Instant::now();

    while !app.should_quit {
        terminal.draw(|f| app.render(f))?;

        // 백그라운드 spec 로드 + 프리로드 처리
        app.poll_bg_loading();

        // 백그라운드 액션 완료 체크
        app.poll_action();

        // 자동 갱신: 현재 탭의 refresh_interval 체크
        let refresh_secs = app.current_refresh_interval();
        if refresh_secs > 0 && last_refresh.elapsed().as_secs() >= refresh_secs as u64 {
            app.refresh_current_tab();
            last_refresh = std::time::Instant::now();
        }

        let poll_ms = if app.bg_loading.is_some() || app.action_running { 50 } else { 200 };
        if event::poll(std::time::Duration::from_millis(poll_ms))? {
            match event::read()? {
                Event::Key(k) => { app.handle_key(k); last_refresh = std::time::Instant::now(); },
                Event::Mouse(m) => app.handle_mouse(m),
                _ => {}
            }
        }
    }

    io::stdout().execute(DisableMouseCapture)?;
    io::stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

/// GitHub Releases에서 최신 버전 태그 조회. 실패 시 None (오프라인 등).
fn check_latest_version() -> Option<String> {
    let output = Command::new("gh")
        .args(["release", "list", "--repo", "dalsoop/mac-app-init",
               "--limit", "1", "--exclude-pre-releases", "--json", "tagName"])
        .output().ok()?;
    if !output.status.success() { return None; }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let releases: Vec<serde_json::Value> = serde_json::from_str(&stdout).ok()?;
    let tag = releases.first()?.get("tagName")?.as_str()?;
    // "v1.0.5" → "1.0.5"
    Some(tag.trim_start_matches('v').to_string())
}
