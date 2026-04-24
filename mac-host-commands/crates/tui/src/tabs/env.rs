use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashMap;
use std::process::Command;

struct EnvEntry {
    key: String,
    value: String,
    encrypted: bool,
}

pub struct EnvTab {
    entries: Vec<EnvEntry>,
    selected: usize,
    show_decrypted: bool,
    decrypt_cache: HashMap<String, String>,
    output: String,
    search: String,
    searching: bool,
    filtered: Vec<usize>,
    env_path: String,
}

impl EnvTab {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            entries: Vec::new(),
            selected: 0,
            show_decrypted: false,
            decrypt_cache: HashMap::new(),
            output: String::new(),
            search: String::new(),
            searching: false,
            filtered: Vec::new(),
            env_path: format!("{}/.env", home),
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.entries.clear();
        self.decrypt_cache.clear();

        let raw = match std::fs::read_to_string(&self.env_path) {
            Ok(content) => content,
            Err(e) => {
                self.output = format!("Error reading {}: {}", self.env_path, e);
                self.apply_filter();
                return Ok(());
            }
        };

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                if key == "DOTENV_PUBLIC_KEY" {
                    continue;
                }
                let value = trimmed[eq_pos + 1..].trim().to_string();
                let encrypted = value.starts_with("encrypted:");
                self.entries.push(EnvEntry {
                    key,
                    value,
                    encrypted,
                });
            }
        }

        self.entries.sort_by(|a, b| a.key.cmp(&b.key));
        self.apply_filter();
        Ok(())
    }

    fn apply_filter(&mut self) {
        let query = self.search.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| query.is_empty() || e.key.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn decrypt_all(&mut self) {
        self.decrypt_cache.clear();
        let output = Command::new("dotenvx")
            .args(["get", "-f", &self.env_path, "--format", "json"])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&stdout) {
                    for (k, v) in map {
                        let val = match v {
                            serde_json::Value::String(s) => s,
                            other => other.to_string(),
                        };
                        self.decrypt_cache.insert(k, val);
                    }
                    self.output = format!("Decrypted {} keys", self.decrypt_cache.len());
                } else {
                    // Fallback: line-by-line
                    for line in stdout.lines() {
                        if let Some(eq_pos) = line.find('=') {
                            let k = line[..eq_pos].trim().to_string();
                            let v = line[eq_pos + 1..].trim().to_string();
                            self.decrypt_cache.insert(k, v);
                        }
                    }
                    self.output = format!("Decrypted {} keys (fallback)", self.decrypt_cache.len());
                }
            }
            Ok(o) => {
                self.output = format!(
                    "Decrypt failed: {}",
                    String::from_utf8_lossy(&o.stderr).trim()
                );
            }
            Err(e) => {
                self.output = format!("dotenvx not found: {}", e);
            }
        }
    }

    fn cached_value(&self, key: &str) -> &str {
        self.decrypt_cache.get(key).map(|s| s.as_str()).unwrap_or("<not decrypted>")
    }

    fn selected_entry(&self) -> Option<&EnvEntry> {
        self.filtered.get(self.selected).and_then(|&i| self.entries.get(i))
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(chunks[0]);

        // Search
        let search_text = if self.searching {
            format!("/{}", self.search)
        } else if self.search.is_empty() {
            format!(" {} env vars", self.filtered.len())
        } else {
            format!("/{} ({} results)", self.search, self.filtered.len())
        };
        let search = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if self.searching {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                })
                .title(" Search "),
        );
        frame.render_widget(search, left[0]);

        // Table
        let visible_height = left[1].height.saturating_sub(2) as usize;
        let scroll = if self.selected >= visible_height {
            self.selected - visible_height + 1
        } else {
            0
        };

        let rows: Vec<Row> = self
            .filtered
            .iter()
            .skip(scroll)
            .take(visible_height)
            .enumerate()
            .map(|(vis_i, &idx)| {
                let entry = &self.entries[idx];
                let is_selected = scroll + vis_i == self.selected;
                let base = if is_selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };

                let lock = if entry.encrypted { "[E]" } else { "[P]" };
                let display_val = if self.show_decrypted && entry.encrypted {
                    self.cached_value(&entry.key).to_string()
                } else if entry.encrypted {
                    "encrypted:****".to_string()
                } else {
                    entry.value.clone()
                };

                Row::new(vec![
                    Cell::from(lock).style(base.fg(if entry.encrypted { Color::Green } else { Color::Yellow })),
                    Cell::from(entry.key.as_str()).style(base),
                    Cell::from(display_val).style(base.fg(if entry.encrypted && !self.show_decrypted {
                        Color::DarkGray
                    } else {
                        Color::White
                    })),
                ])
                .style(base)
            })
            .collect();

        let header = Row::new(vec!["", "Key", "Value"])
            .style(Style::default().fg(Color::Yellow).bold());

        let table = Table::new(rows, [
            Constraint::Length(4),
            Constraint::Percentage(30),
            Constraint::Percentage(60),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(
                    " ~/.env ({}) {} ",
                    self.entries.len(),
                    if self.show_decrypted { "[DECRYPTED]" } else { "" }
                )),
        );
        frame.render_widget(table, left[1]);

        // Right: details + actions + output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Length(10), Constraint::Min(0)])
            .split(chunks[1]);

        // Detail
        let detail_text = if let Some(entry) = self.selected_entry() {
            let val_display = if self.show_decrypted && entry.encrypted {
                self.cached_value(&entry.key).to_string()
            } else if entry.encrypted {
                let truncated: String = entry.value.chars().take(60).collect();
                format!("{}...", truncated)
            } else {
                entry.value.clone()
            };

            format!(
                "Key: {}\nEncrypted: {}\n\nValue:\n{}",
                entry.key, entry.encrypted, val_display,
            )
        } else {
            "No entry selected".to_string()
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

        // Actions
        let actions = Paragraph::new(vec![
            Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  d", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Toggle decrypt (one-shot)"),
            ]),
            Line::from(vec![
                Span::styled("  r", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Refresh"),
            ]),
            Line::from(vec![
                Span::styled("  e", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Re-encrypt all"),
            ]),
            Line::from(vec![
                Span::styled("  /", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Search"),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Actions "),
        );
        frame.render_widget(actions, right[1]);

        // Output
        let output = Paragraph::new(self.output.as_str())
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Output "),
            );
        frame.render_widget(output, right[2]);
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.searching {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.search.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.search.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Char('d') => {
                if self.show_decrypted {
                    self.show_decrypted = false;
                    self.decrypt_cache.clear();
                    self.output = "Decrypted view OFF".to_string();
                } else {
                    self.decrypt_all();
                    self.show_decrypted = true;
                }
            }
            KeyCode::Char('r') => {
                self.show_decrypted = false;
                self.decrypt_cache.clear();
                self.load().await?;
                self.output = "Refreshed.".to_string();
            }
            KeyCode::Char('e') => {
                let output = Command::new("dotenvx")
                    .args(["encrypt", "-f", &self.env_path])
                    .output()?;
                self.output = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr),
                );
                self.show_decrypted = false;
                self.decrypt_cache.clear();
                self.load().await?;
            }
            _ => {}
        }
        Ok(())
    }
}
