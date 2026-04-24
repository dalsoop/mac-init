use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::services::mac_cmd;

pub struct FilesTab {
    status_output: String,
    action_output: String,
    scroll: u16,
    selected_action: usize,
}

const ACTIONS: &[(&str, &[&str], &str)] = &[
    ("Organize Downloads", &["files", "organize"], "파일 자동 분류"),
    ("Cleanup Temp", &["files", "cleanup-temp"], "30일 이상 임시파일 정리"),
    ("SD Backup Status", &["files", "sd-status"], "SD 카드 백업 상태"),
    ("SD Backup Run", &["files", "sd-run"], "SD 카드 백업 실행"),
    ("Enable Auto-organize", &["files", "setup-auto"], "자동 분류 활성화"),
    ("Disable Auto-organize", &["files", "disable-auto"], "자동 분류 비활성화"),
    ("Lint Files", &["files", "lint"], "파일 구조 검증"),
];

impl FilesTab {
    pub fn new() -> Self {
        Self {
            status_output: String::new(),
            action_output: String::new(),
            scroll: 0,
            selected_action: 0,
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.status_output = mac_cmd::run(&["files", "status"])?;
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: status
        let status_lines: Vec<Line> = self
            .status_output
            .lines()
            .map(|line| {
                let style = if line.contains('✓') {
                    Style::default().fg(Color::Green)
                } else if line.contains('✗') {
                    Style::default().fg(Color::Red)
                } else if line.trim().starts_with("===") {
                    Style::default().fg(Color::Cyan).bold()
                } else {
                    Style::default().fg(Color::Gray)
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect();

        let status = Paragraph::new(status_lines)
            .scroll((self.scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Files Status "),
            );
        frame.render_widget(status, chunks[0]);

        // Right: actions list + output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(ACTIONS.len() as u16 + 4), Constraint::Min(0)])
            .split(chunks[1]);

        // Actions
        let action_items: Vec<ListItem> = ACTIONS
            .iter()
            .enumerate()
            .map(|(i, (name, _, desc))| {
                let style = if i == self.selected_action {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", name), style.bold()),
                    Span::styled(format!("  {}", desc), style.fg(Color::Gray)),
                ]))
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
        let output = Paragraph::new(self.action_output.as_str())
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
                self.selected_action = self.selected_action.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_action + 1 < ACTIONS.len() {
                    self.selected_action += 1;
                }
            }
            KeyCode::Enter => {
                let (name, args, _) = ACTIONS[self.selected_action];
                self.action_output = format!("Running {}...\n", name);
                let result = mac_cmd::run(args)?;
                self.action_output.push_str(&result);
                // Refresh status after action
                self.status_output = mac_cmd::run(&["files", "status"])?;
            }
            KeyCode::Char('r') => {
                self.load().await?;
                self.action_output = "Refreshed.".to_string();
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
