use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::services::mac_cmd;

#[derive(Clone)]
struct MountEntry {
    name: String,
    source: String,
    target: String,
    method: String,
    mounted: bool,
}

pub struct MountsTab {
    mounts: Vec<MountEntry>,
    selected: usize,
    output: String,
}

impl MountsTab {
    pub fn new() -> Self {
        Self {
            mounts: Vec::new(),
            selected: 0,
            output: String::new(),
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        let raw = mac_cmd::run(&["mount", "status"])?;
        self.mounts.clear();

        for line in raw.lines() {
            let trimmed = line.trim();
            // Parse: [name] user@host:path -> /Volumes/x (method) ✓/✗
            if trimmed.starts_with('[') && trimmed.contains("->") {
                let mounted = trimmed.contains('✓');
                // Extract name
                let name = trimmed
                    .split(']')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('[')
                    .to_string();

                let rest = trimmed.split(']').nth(1).unwrap_or("").trim();
                let parts: Vec<&str> = rest.split("->").collect();
                let source = parts.first().unwrap_or(&"").trim().to_string();
                let target_part = parts.get(1).unwrap_or(&"").trim();

                // Extract target and method
                let (target, method) = if let Some(paren_start) = target_part.find('(') {
                    let t = target_part[..paren_start].trim().to_string();
                    let m = target_part[paren_start..]
                        .trim_matches(|c| c == '(' || c == ')')
                        .split(')')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    (t, m)
                } else {
                    (target_part.to_string(), String::new())
                };

                self.mounts.push(MountEntry {
                    name,
                    source,
                    target,
                    method,
                    mounted,
                });
            }
        }

        if self.selected >= self.mounts.len() {
            self.selected = self.mounts.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        // Left: mount table
        let header = Row::new(vec!["Status", "Name", "Target", "Type"])
            .style(Style::default().fg(Color::Yellow).bold());

        let rows: Vec<Row> = self
            .mounts
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let is_selected = i == self.selected;
                let status = if m.mounted { "✓ mounted" } else { "✗ stopped" };
                let status_style = if m.mounted {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                let base = if is_selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    Cell::from(status).style(if is_selected { base } else { status_style }),
                    Cell::from(m.name.as_str()).style(base),
                    Cell::from(m.target.as_str()).style(base),
                    Cell::from(m.method.as_str()).style(base),
                ])
            })
            .collect();

        let table = Table::new(rows, [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(8),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" Mounts ({}) ", self.mounts.len())),
        );
        frame.render_widget(table, chunks[0]);

        // Right: details + actions + output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Min(0),
            ])
            .split(chunks[1]);

        // Details
        let detail_text = if let Some(m) = self.mounts.get(self.selected) {
            format!(
                "Name: {}\nSource: {}\nTarget: {}\nMethod: {}\nStatus: {}",
                m.name,
                m.source,
                m.target,
                m.method,
                if m.mounted { "Mounted" } else { "Not mounted" },
            )
        } else {
            "No mounts configured".to_string()
        };

        let mut lines: Vec<Line> = detail_text.lines().map(|l| Line::from(l.to_string())).collect();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  m", Style::default().fg(Color::Cyan).bold()),
            Span::raw(": mount  "),
            Span::styled("u", Style::default().fg(Color::Cyan).bold()),
            Span::raw(": unmount  "),
            Span::styled("a", Style::default().fg(Color::Cyan).bold()),
            Span::raw(": mount all  "),
            Span::styled("r", Style::default().fg(Color::Cyan).bold()),
            Span::raw(": refresh"),
        ]));

        let details = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Details "),
            );
        frame.render_widget(details, right[0]);

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
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.mounts.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Char('m') => {
                // Mount selected
                if let Some(m) = self.mounts.get(self.selected) {
                    if !m.mounted {
                        let name = m.name.clone();
                        self.output = mac_cmd::run(&["mount", "up", &name])?;
                        self.load().await?;
                    }
                }
            }
            KeyCode::Char('u') => {
                // Unmount selected
                if let Some(m) = self.mounts.get(self.selected) {
                    if m.mounted {
                        let name = m.name.clone();
                        self.output = mac_cmd::run(&["mount", "down", &name])?;
                        self.load().await?;
                    }
                }
            }
            KeyCode::Char('a') => {
                // Mount all
                self.output = mac_cmd::run(&["mount", "up-all"])?;
                self.load().await?;
            }
            KeyCode::Char('r') => {
                self.load().await?;
                self.output = "Refreshed.".to_string();
            }
            _ => {}
        }
        Ok(())
    }
}
