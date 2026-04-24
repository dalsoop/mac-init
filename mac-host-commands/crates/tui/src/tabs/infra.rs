use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::services::mac_cmd;

#[derive(Clone, Copy, PartialEq)]
enum InfraView {
    Overview,
    Proxmox,
    Synology,
    Veil,
    Workspace,
}

impl InfraView {
    const ALL: &[InfraView] = &[
        InfraView::Overview,
        InfraView::Proxmox,
        InfraView::Synology,
        InfraView::Veil,
        InfraView::Workspace,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Proxmox => "Proxmox",
            Self::Synology => "Synology",
            Self::Veil => "Veil",
            Self::Workspace => "Workspace",
        }
    }

    fn command(&self) -> &[&str] {
        match self {
            Self::Overview => &["status"],
            Self::Proxmox => &["proxmox", "status"],
            Self::Synology => &["synology", "status"],
            Self::Veil => &["veil", "status"],
            Self::Workspace => &["workspace", "status"],
        }
    }
}

pub struct InfraTab {
    view: InfraView,
    view_index: usize,
    content: String,
    scroll: u16,
    output: String,
}

impl InfraTab {
    pub fn new() -> Self {
        Self {
            view: InfraView::Overview,
            view_index: 0,
            content: String::new(),
            scroll: 0,
            output: String::new(),
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.content = mac_cmd::run(self.view.command())?;
        self.scroll = 0;
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // Sub-tab bar
        let titles: Vec<&str> = InfraView::ALL.iter().map(|v| v.label()).collect();
        let tabs = Tabs::new(titles)
            .select(self.view_index)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(Style::default().fg(Color::Magenta).bold().bg(Color::DarkGray))
            .divider("|");
        frame.render_widget(tabs, chunks[0]);

        let content_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(chunks[1]);

        // Left: content
        let lines: Vec<Line> = self
            .content
            .lines()
            .map(|line| {
                let style = if line.contains('✓') {
                    Style::default().fg(Color::Green)
                } else if line.contains('✗') {
                    Style::default().fg(Color::Red)
                } else if line.trim().starts_with("===") || line.trim().starts_with('[') {
                    Style::default().fg(Color::Cyan).bold()
                } else if line.contains("running") {
                    Style::default().fg(Color::Green)
                } else if line.contains("stopped") {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Gray)
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" {} ", self.view.label())),
            );
        frame.render_widget(paragraph, content_area[0]);

        // Right: actions + output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(content_area[1]);

        let actions = Paragraph::new(vec![
            Line::from(Span::styled(" Navigation", Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ←→", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Switch view"),
            ]),
            Line::from(vec![
                Span::styled("  r", Style::default().fg(Color::Cyan).bold()),
                Span::raw("   Refresh"),
            ]),
            Line::from(vec![
                Span::styled("  j/k", Style::default().fg(Color::Cyan).bold()),
                Span::raw(" Scroll"),
            ]),
            Line::from(""),
            Line::from(Span::styled(" Proxmox Actions", Style::default().fg(Color::Yellow).bold())),
            Line::from(vec![
                Span::styled("  l", Style::default().fg(Color::Cyan).bold()),
                Span::raw("   LXC list"),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Actions "),
        );
        frame.render_widget(actions, right[0]);

        let output = Paragraph::new(self.output.as_str())
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Output "),
            );
        frame.render_widget(output, right[1]);
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                if self.view_index > 0 {
                    self.view_index -= 1;
                    self.view = InfraView::ALL[self.view_index];
                    self.load().await?;
                }
            }
            KeyCode::Right | KeyCode::Char('l') if key.modifiers.is_empty() => {
                if self.view_index + 1 < InfraView::ALL.len() {
                    self.view_index += 1;
                    self.view = InfraView::ALL[self.view_index];
                    self.load().await?;
                }
            }
            KeyCode::Char('r') => {
                self.load().await?;
                self.output = "Refreshed.".to_string();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Char('d') => {
                self.scroll = self.scroll.saturating_add(10);
            }
            KeyCode::Char('u') => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            _ => {}
        }
        Ok(())
    }
}
