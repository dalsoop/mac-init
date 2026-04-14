use std::io;

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::tabs::TabId;
#[cfg(domain = "brew")]
use crate::tabs::brew::BrewTab;
use crate::tabs::configs::ConfigsTab;
#[cfg(domain = "defaults")]
use crate::tabs::defaults::DefaultsTab;
use crate::tabs::env::EnvTab;
use crate::tabs::host::HostTab;
use crate::tabs::containers::ContainersTab;
use crate::tabs::status::StatusTab;
use crate::tabs::store::StoreTab;
use crate::ui::tabbar::render_tabbar;

pub struct App {
    active_tab: TabId,
    status_tab: StatusTab,
    #[cfg(domain = "brew")]
    brew_tab: BrewTab,
    env_tab: EnvTab,
    containers_tab: ContainersTab,
    configs_tab: ConfigsTab,
    host_tab: HostTab,
    #[cfg(domain = "defaults")]
    defaults_tab: DefaultsTab,
    store_tab: StoreTab,
    should_quit: bool,
    loading: bool,
    loading_msg: String,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            active_tab: TabId::Status,
            status_tab: StatusTab::new(),
            #[cfg(domain = "brew")]
            brew_tab: BrewTab::new(),
            env_tab: EnvTab::new(),
            containers_tab: ContainersTab::new(),
            configs_tab: ConfigsTab::new(),
            host_tab: HostTab::new(),
            #[cfg(domain = "defaults")]
            defaults_tab: DefaultsTab::new(),
            store_tab: StoreTab::new(),
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

        // Load tabs
        self.loading_msg = "Loading env...".to_string();
        terminal.draw(|frame| self.render(frame))?;
        self.env_tab.load().await?;

        self.loading_msg = "Loading configs...".to_string();
        terminal.draw(|frame| self.render(frame))?;
        self.configs_tab.load().await?;

        self.loading_msg = "Loading services...".to_string();
        terminal.draw(|frame| self.render(frame))?;
        self.containers_tab.load().await?;

        self.loading_msg = "Loading status...".to_string();
        terminal.draw(|frame| self.render(frame))?;
        self.status_tab.load().await?;

        #[cfg(domain = "defaults")]
        {
            self.loading_msg = "Loading defaults...".to_string();
            terminal.draw(|frame| self.render(frame))?;
            self.defaults_tab.load().await?;
        }

        #[cfg(domain = "brew")]
        {
            self.loading_msg = "Loading brew...".to_string();
            terminal.draw(|frame| self.render(frame))?;
            self.brew_tab.load().await?;
        }

        self.loading_msg = "Loading store...".to_string();
        terminal.draw(|frame| self.render(frame))?;
        self.store_tab.load().await?;

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
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        render_tabbar(frame, chunks[0], &self.active_tab);

        match self.active_tab {
            TabId::Status => self.status_tab.render(frame, chunks[1]),
            #[cfg(domain = "brew")]
            TabId::Brew => self.brew_tab.render(frame, chunks[1]),
            TabId::Env => self.env_tab.render(frame, chunks[1]),
            TabId::Containers => self.containers_tab.render(frame, chunks[1]),
            TabId::Configs => self.configs_tab.render(frame, chunks[1]),
            TabId::Host => self.host_tab.render(frame, chunks[1]),
            #[cfg(domain = "defaults")]
            TabId::Defaults => self.defaults_tab.render(frame, chunks[1]),
            TabId::Store => self.store_tab.render(frame, chunks[1]),
        }

        let tab_hints = match self.active_tab {
            TabId::Status => "r:refresh enter:action",
            #[cfg(domain = "brew")]
            TabId::Brew => "/:search u:update r:remove",
            TabId::Env => "d:decrypt /:search e:encrypt",
            TabId::Containers => "l:load s:stop r:restart",
            TabId::Configs => "e:edit d/u:scroll",
            TabId::Host => "h/l:switch view r:refresh",
            #[cfg(domain = "defaults")]
            TabId::Defaults => "enter:open esc:back",
            TabId::Store => "i:install d:remove u:update",
        };
        let tab_count = TabId::count();
        let status = Line::from(vec![
            Span::styled(" q", Style::default().fg(Color::Yellow).bold()),
            Span::raw(":quit  "),
            Span::styled(format!("1-{}", tab_count), Style::default().fg(Color::Yellow).bold()),
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
                    KeyCode::Char(c @ '1'..='9') => {
                        let n = (c as usize) - ('1' as usize);
                        if let Some(tab) = TabId::from_num(n) {
                            self.active_tab = tab;
                        }
                        return Ok(());
                    }
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
                    #[cfg(domain = "brew")]
                    TabId::Brew => self.brew_tab.handle_key(key).await?,
                    TabId::Env => self.env_tab.handle_key(key).await?,
                    TabId::Containers => self.containers_tab.handle_key(key).await?,
                    TabId::Configs => self.configs_tab.handle_key(key).await?,
                    TabId::Host => self.host_tab.handle_key(key).await?,
                    #[cfg(domain = "defaults")]
                    TabId::Defaults => self.defaults_tab.handle_key(key).await?,
                    TabId::Store => self.store_tab.handle_key(key).await?,
                }
            }
        }
        Ok(())
    }
}
