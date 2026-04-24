use std::io;

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::tabs::{
    TabId,
    brew::BrewTab,
    configs::ConfigsTab,
    defaults::DefaultsTab,
    env::EnvTab,
    infra::InfraTab,
    services::ServicesTab,
    status::StatusTab,
};
use crate::ui::tabbar::render_tabbar;

pub struct App {
    active_tab: TabId,
    status_tab: StatusTab,
    brew_tab: BrewTab,
    env_tab: EnvTab,
    services_tab: ServicesTab,
    configs_tab: ConfigsTab,
    infra_tab: InfraTab,
    defaults_tab: DefaultsTab,
    should_quit: bool,
    loading: bool,
    loading_msg: String,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            active_tab: TabId::Status,
            status_tab: StatusTab::new(),
            brew_tab: BrewTab::new(),
            env_tab: EnvTab::new(),
            services_tab: ServicesTab::new(),
            configs_tab: ConfigsTab::new(),
            infra_tab: InfraTab::new(),
            defaults_tab: DefaultsTab::new(),
            should_quit: false,
            loading: true,
            loading_msg: "Starting...".to_string(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        io::stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = ratatui::Terminal::new(backend)?;
        terminal.clear()?;

        // Load fast tabs first, brew last
        let steps = ["env", "configs", "services", "status", "defaults", "brew"];

        for name in &steps {
            self.loading_msg = format!("Loading {}...", name);
            terminal.draw(|frame| self.render(frame))?;

            match *name {
                "env" => self.env_tab.load().await?,
                "configs" => self.configs_tab.load().await?,
                "services" => self.services_tab.load().await?,
                "status" => self.status_tab.load().await?,
                "defaults" => self.defaults_tab.load().await?,
                "brew" => self.brew_tab.load().await?,
                _ => {}
            }
        }

        self.loading = false;

        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events().await?;
        }

        io::stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        if self.loading {
            let area = frame.area();
            let center = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Percentage(40),
                ])
                .split(area);
            frame.render_widget(
                Paragraph::new("mac-init")
                    .style(Style::default().fg(Color::Cyan).bold())
                    .alignment(Alignment::Center),
                center[1],
            );
            frame.render_widget(
                Paragraph::new(self.loading_msg.as_str())
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                center[3],
            );
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // tabbar
                Constraint::Min(0),   // content
                Constraint::Length(1), // status bar
            ])
            .split(frame.area());

        render_tabbar(frame, chunks[0], &self.active_tab);

        match self.active_tab {
            TabId::Status => self.status_tab.render(frame, chunks[1]),
            TabId::Brew => self.brew_tab.render(frame, chunks[1]),
            TabId::Env => self.env_tab.render(frame, chunks[1]),
            TabId::Services => self.services_tab.render(frame, chunks[1]),
            TabId::Configs => self.configs_tab.render(frame, chunks[1]),
            TabId::Infra => self.infra_tab.render(frame, chunks[1]),
            TabId::Defaults => self.defaults_tab.render(frame, chunks[1]),
        }

        let tab_hints = match self.active_tab {
            TabId::Status => "r:refresh enter:action",
            TabId::Brew => "/:search u:update r:remove",
            TabId::Env => "d:decrypt /:search e:encrypt",
            TabId::Services => "l:load s:stop",
            TabId::Configs => "e:edit d/u:scroll",
            TabId::Infra => "h/l:switch view r:refresh",
            TabId::Defaults => "enter:open esc:back",
        };
        let status = Line::from(vec![
            Span::styled(" q", Style::default().fg(Color::Yellow).bold()),
            Span::raw(":quit  "),
            Span::styled("1-7", Style::default().fg(Color::Yellow).bold()),
            Span::raw(":tabs  "),
            Span::styled("j/k", Style::default().fg(Color::Yellow).bold()),
            Span::raw(":nav  "),
            Span::styled(tab_hints, Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(
            status.style(Style::default().bg(Color::DarkGray)),
            chunks[2],
        );
    }

    async fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('q') if key.modifiers.is_empty() => {
                        self.should_quit = true;
                        return Ok(());
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.should_quit = true;
                        return Ok(());
                    }
                    KeyCode::Char('1') => { self.active_tab = TabId::Status; return Ok(()); }
                    KeyCode::Char('2') => { self.active_tab = TabId::Brew; return Ok(()); }
                    KeyCode::Char('3') => { self.active_tab = TabId::Env; return Ok(()); }
                    KeyCode::Char('4') => { self.active_tab = TabId::Services; return Ok(()); }
                    KeyCode::Char('5') => { self.active_tab = TabId::Configs; return Ok(()); }
                    KeyCode::Char('6') => { self.active_tab = TabId::Infra; return Ok(()); }
                    KeyCode::Char('7') => { self.active_tab = TabId::Defaults; return Ok(()); }
                    KeyCode::Tab => {
                        self.active_tab = self.active_tab.next();
                        return Ok(());
                    }
                    KeyCode::BackTab => {
                        self.active_tab = self.active_tab.prev();
                        return Ok(());
                    }
                    _ => {}
                }

                match self.active_tab {
                    TabId::Status => self.status_tab.handle_key(key).await?,
                    TabId::Brew => self.brew_tab.handle_key(key).await?,
                    TabId::Env => self.env_tab.handle_key(key).await?,
                    TabId::Services => self.services_tab.handle_key(key).await?,
                    TabId::Configs => self.configs_tab.handle_key(key).await?,
                    TabId::Infra => self.infra_tab.handle_key(key).await?,
                    TabId::Defaults => self.defaults_tab.handle_key(key).await?,
                }
            }
        }
        Ok(())
    }
}
