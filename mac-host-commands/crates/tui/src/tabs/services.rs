use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::models::LaunchAgent;
use crate::services;

pub struct ServicesTab {
    agents: Vec<LaunchAgent>,
    selected: usize,
    output: String,
}

impl ServicesTab {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected: 0,
            output: String::new(),
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.agents = services::launch_agents::scan_agents()?;
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        // Left: agents table
        let visible_height = chunks[0].height.saturating_sub(2) as usize;
        let scroll = self.selected.saturating_sub(visible_height.saturating_sub(1));

        let rows: Vec<Row> = self
            .agents
            .iter()
            .skip(scroll)
            .take(visible_height)
            .enumerate()
            .map(|(vis_i, agent)| {
                let is_selected = scroll + vis_i == self.selected;
                let status_style = if agent.running {
                    Style::default().fg(Color::Green)
                } else if agent.loaded {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Red)
                };
                let status = if agent.running {
                    format!("running ({})", agent.pid.unwrap_or(0))
                } else if agent.loaded {
                    "loaded".to_string()
                } else {
                    "stopped".to_string()
                };
                let base_style = if is_selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    Cell::from(status).style(if is_selected { base_style } else { status_style }),
                    Cell::from(agent.label.as_str()).style(base_style),
                ])
            })
            .collect();

        let header = Row::new(vec!["Status", "Label"])
            .style(Style::default().fg(Color::Yellow).bold());

        let table = Table::new(rows, [
            Constraint::Length(18),
            Constraint::Min(20),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" LaunchAgents ({}) ", self.agents.len())),
        );
        frame.render_widget(table, chunks[0]);

        // Right: details + output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let detail_text = if let Some(agent) = self.agents.get(self.selected) {
            let status = if agent.running {
                format!("Running (PID: {})", agent.pid.unwrap_or(0))
            } else if agent.loaded {
                "Loaded (not running)".to_string()
            } else {
                "Stopped".to_string()
            };
            format!(
                "Label: {}\nStatus: {}\nPath: {}\nProgram: {}",
                agent.label,
                status,
                agent.path.display(),
                agent.program,
            )
        } else {
            "No agent selected".to_string()
        };
        let details = Paragraph::new(detail_text)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Details "),
            );
        frame.render_widget(details, right[0]);

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
                if self.selected + 1 < self.agents.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Char('l') => {
                // Load/start agent
                if let Some(agent) = self.agents.get(self.selected) {
                    if !agent.loaded {
                        let label = agent.label.clone();
                        let path = agent.path.clone();
                        self.output = services::launch_agents::load_agent(&label, &path)?;
                        self.load().await?;
                    }
                }
            }
            KeyCode::Char('s') => {
                // Stop/unload agent
                if let Some(agent) = self.agents.get(self.selected) {
                    if agent.loaded {
                        let label = agent.label.clone();
                        let path = agent.path.clone();
                        self.output = services::launch_agents::unload_agent(&label, &path)?;
                        self.load().await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
