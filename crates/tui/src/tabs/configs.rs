use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use mac_host_core::models::config_entry::{ConfigEntry, ConfigCategory};
use mac_host_core::dotfiles;

pub struct ConfigsTab {
    configs: Vec<ConfigEntry>,
    selected: usize,
    preview: String,
    preview_scroll: u16,
}

impl ConfigsTab {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            selected: 0,
            preview: String::new(),
            preview_scroll: 0,
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.configs = dotfiles::scan_configs();
        self.update_preview();
        Ok(())
    }

    fn update_preview(&mut self) {
        self.preview_scroll = 0;
        if let Some(entry) = self.configs.get(self.selected) {
            if entry.path.is_file() {
                self.preview = dotfiles::read_config(&entry.path)
                    .unwrap_or_else(|| "Error reading file".to_string());
            } else if entry.path.is_dir() {
                // List directory contents
                let mut contents = String::new();
                if let Ok(entries) = std::fs::read_dir(&entry.path) {
                    for e in entries.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        let meta = e.metadata().ok();
                        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                        contents.push_str(&format!("{:<30} {:>8}\n", name, format_size(size)));
                    }
                }
                self.preview = contents;
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Left: config list
        let visible_height = chunks[0].height.saturating_sub(2) as usize;
        let scroll = self.selected.saturating_sub(visible_height.saturating_sub(1));

        let rows: Vec<Row> = self
            .configs
            .iter()
            .skip(scroll)
            .take(visible_height)
            .enumerate()
            .map(|(vis_i, entry)| {
                let style = if scroll + vis_i == self.selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };
                let cat_color = match entry.category {
                    ConfigCategory::Shell => Color::Green,
                    ConfigCategory::Git => Color::Red,
                    ConfigCategory::Ssh => Color::Yellow,
                    ConfigCategory::Editor => Color::Blue,
                    ConfigCategory::Terminal => Color::Magenta,
                    ConfigCategory::Keyboard => Color::Cyan,
                    ConfigCategory::Other => Color::Gray,
                };
                Row::new(vec![
                    Cell::from(entry.category.to_string()).style(Style::default().fg(cat_color)),
                    Cell::from(entry.name.as_str()),
                    Cell::from(format_size(entry.size_bytes)),
                    Cell::from(entry.modified.as_str()),
                ])
                .style(style)
            })
            .collect();

        let header = Row::new(vec!["Cat", "Name", "Size", "Modified"])
            .style(Style::default().fg(Color::Yellow).bold());

        let table = Table::new(rows, [
            Constraint::Length(10),
            Constraint::Min(15),
            Constraint::Length(10),
            Constraint::Length(12),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" Configs ({}) ", self.configs.len())),
        );
        frame.render_widget(table, chunks[0]);

        // Right: preview
        let title = self
            .configs
            .get(self.selected)
            .map(|e| format!(" {} ", e.path.display()))
            .unwrap_or_else(|| " Preview ".to_string());

        let preview = Paragraph::new(self.preview.as_str())
            .scroll((self.preview_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(title),
            );
        frame.render_widget(preview, chunks[1]);
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.update_preview();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.configs.len() {
                    self.selected += 1;
                    self.update_preview();
                }
            }
            KeyCode::Char('d') => {
                self.preview_scroll = self.preview_scroll.saturating_add(10);
            }
            KeyCode::Char('u') => {
                self.preview_scroll = self.preview_scroll.saturating_sub(10);
            }
            KeyCode::Char('e') => {
                // Open in $EDITOR
                if let Some(entry) = self.configs.get(self.selected) {
                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                    let _ = std::process::Command::new(&editor)
                        .arg(&entry.path)
                        .status();
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    }
}
