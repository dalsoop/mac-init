use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use mac_host_core::common;

struct Section {
    title: String,
    lines: Vec<(String, Option<bool>)>, // (text, ok/fail/info)
}

pub struct StatusTab {
    sections: Vec<Section>,
    scroll: u16,
    output: String,
    selected_action: usize,
}

const ACTIONS: &[(&str, &[&str])] = &[
    ("Refresh all", &["status"]),
    ("Mount all", &["mount", "up-all"]),
    ("Unmount all", &["mount", "down-all"]),
    ("Organize files", &["files", "organize"]),
    ("Cleanup temp", &["files", "cleanup-temp"]),
    ("SD backup run", &["files", "sd-run"]),
];

impl StatusTab {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            scroll: 0,
            output: String::new(),
            selected_action: 0,
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.sections.clear();

        // Load all status sources
        let sources: Vec<(&str, &[&str])> = vec![
            ("status", &["status"] as &[&str]),
            ("mount", &["mount", "status"]),
        ];

        for (_, args) in &sources {
            let raw = common::run_self(args);
            self.parse_output(&raw);
        }

        Ok(())
    }

    fn parse_output(&mut self, raw: &str) {
        let mut current: Option<Section> = None;

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("===") && trimmed.ends_with("===") {
                if let Some(section) = current.take() {
                    self.sections.push(section);
                }
                let title = trimmed.trim_matches('=').trim().to_string();
                current = Some(Section {
                    title,
                    lines: Vec::new(),
                });
            } else if trimmed.starts_with("──") {
                continue;
            } else if !trimmed.is_empty() {
                let ok = if trimmed.contains('✓') {
                    Some(true)
                } else if trimmed.contains('✗') {
                    Some(false)
                } else {
                    None
                };
                if let Some(ref mut section) = current {
                    section.lines.push((line.to_string(), ok));
                } else {
                    // Lines before first section header
                    let section = Section {
                        title: "Info".to_string(),
                        lines: vec![(line.to_string(), ok)],
                    };
                    self.sections.push(section);
                }
            }
        }
        if let Some(section) = current.take() {
            self.sections.push(section);
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Left: all status sections
        let mut lines: Vec<Line> = Vec::new();
        let mut ok_count = 0u32;
        let mut fail_count = 0u32;

        for section in &self.sections {
            lines.push(Line::from(Span::styled(
                format!(" {} ", section.title),
                Style::default().fg(Color::Cyan).bold(),
            )));
            for (text, ok) in &section.lines {
                let style = match ok {
                    Some(true) => { ok_count += 1; Style::default().fg(Color::Green) }
                    Some(false) => { fail_count += 1; Style::default().fg(Color::Red) }
                    None => Style::default().fg(Color::Gray),
                };
                lines.push(Line::from(Span::styled(text.as_str(), style)));
            }
            lines.push(Line::from(""));
        }

        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(
                        " Dashboard  [{}✓ {}✗] ",
                        ok_count, fail_count,
                    )),
            );
        frame.render_widget(paragraph, chunks[0]);

        // Right: summary + actions + output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(ACTIONS.len() as u16 + 3),
                Constraint::Min(0),
            ])
            .split(chunks[1]);

        // Actions
        let action_items: Vec<ListItem> = ACTIONS
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                let style = if i == self.selected_action {
                    Style::default().bg(Color::DarkGray).fg(Color::White).bold()
                } else {
                    Style::default().fg(Color::Gray)
                };
                ListItem::new(format!("  {} ", name)).style(style)
            })
            .collect();

        let actions = List::new(action_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Actions (Enter to run) "),
        );
        frame.render_widget(actions, right[0]);

        // Output
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
            KeyCode::Char('r') => {
                self.output = "Refreshing...".to_string();
                self.load().await?;
                self.output = "Refreshed.".to_string();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected_action = (self.selected_action + 1).min(ACTIONS.len() - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_action = self.selected_action.saturating_sub(1);
            }
            KeyCode::Enter => {
                let (name, args) = ACTIONS[self.selected_action];
                self.output = format!("Running {}...\n", name);
                let result = common::run_self(args);
                self.output.push_str(&result);
                if name == "Refresh all" {
                    self.load().await?;
                }
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
