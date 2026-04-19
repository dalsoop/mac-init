mod app;
mod registry;
mod spec;
mod widgets;

use app::App;
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

    // 1차(그룹 목록)에 필요한 것만 빠르게 로드. spec은 나중에.
    app.load_fast();

    let mut last_refresh = std::time::Instant::now();

    while !app.should_quit {
        terminal.draw(|f| app.render(f))?;

        // pending_load: 백그라운드 스레드에서 spec 로드
        if let Some(idx) = app.pending_load.take() {
            if app.bg_loading.is_none() {
                let domain = app.domains[idx].clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let spec = crate::registry::fetch_spec(&domain);
                    let _ = tx.send((idx, spec));
                });
                app.bg_loading = Some(rx);
            }
        }
        // 백그라운드 로드 완료 체크
        if let Some(ref rx) = app.bg_loading {
            if let Ok((idx, spec)) = rx.try_recv() {
                app.specs[idx] = spec;
                app.bg_loading = None;
            }
        }

        // 자동 갱신: 현재 탭의 refresh_interval 체크
        let refresh_secs = app.current_refresh_interval();
        if refresh_secs > 0 && last_refresh.elapsed().as_secs() >= refresh_secs as u64 {
            app.refresh_current_tab();
            last_refresh = std::time::Instant::now();
        }

        let poll_ms = if app.bg_loading.is_some() { 50 } else { 200 };
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
