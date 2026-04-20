use mac_host_tui::app::App;
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

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
