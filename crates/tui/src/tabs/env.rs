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

enum Mode {
    Normal,
    Search,
    EditValue { idx: usize, buf: String },
    AddKey { buf: String },
    AddValue { key: String, buf: String },
}

pub struct EnvTab {
    entries: Vec<EnvEntry>,
    selected: usize,
    show_decrypted: bool,
    decrypt_cache: HashMap<String, String>,
    output: String,
    search: String,
    filtered: Vec<usize>,
    env_path: String,
    mode: Mode,
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
            filtered: Vec::new(),
            env_path: format!("{}/.env", home),
            mode: Mode::Normal,
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
                self.entries.push(EnvEntry { key, value, encrypted });
            }
        }

        self.entries.sort_by(|a, b| a.key.cmp(&b.key));
        self.apply_filter();
        Ok(())
    }

    fn apply_filter(&mut self) {
        let query = self.search.to_lowercase();
        self.filtered = self.entries.iter().enumerate()
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
                }
            }
            Ok(o) => self.output = format!("Decrypt failed: {}", String::from_utf8_lossy(&o.stderr).trim()),
            Err(e) => self.output = format!("dotenvx not found: {}", e),
        }
    }

    fn cached_value(&self, key: &str) -> &str {
        self.decrypt_cache.get(key).map(|s| s.as_str()).unwrap_or("<not decrypted>")
    }

    fn selected_entry(&self) -> Option<&EnvEntry> {
        self.filtered.get(self.selected).and_then(|&i| self.entries.get(i))
    }

    fn dotenvx_set(&self, key: &str, value: &str) -> String {
        let output = Command::new("dotenvx")
            .args(["set", key, value, "-f", &self.env_path])
            .output();
        match output {
            Ok(o) if o.status.success() => format!("✓ {} 저장 + 암호화 완료", key),
            Ok(o) => format!("✗ 저장 실패: {}", String::from_utf8_lossy(&o.stderr).trim()),
            Err(e) => format!("✗ dotenvx 실행 실패: {}", e),
        }
    }

    fn dotenvx_delete(&self, key: &str) -> String {
        // dotenvx doesn't have delete, so we manually remove from .env
        let content = std::fs::read_to_string(&self.env_path).unwrap_or_default();
        let lines: Vec<&str> = content.lines()
            .filter(|l| {
                let trimmed = l.trim();
                !trimmed.starts_with(&format!("{}=", key))
            })
            .collect();
        if let Err(e) = std::fs::write(&self.env_path, lines.join("\n") + "\n") {
            return format!("✗ 삭제 실패: {}", e);
        }
        // Re-encrypt
        let _ = Command::new("dotenvx")
            .args(["encrypt", "-f", &self.env_path])
            .output();
        format!("✓ {} 삭제 + 재암호화 완료", key)
    }

    fn dotenvx_encrypt(&self) -> String {
        let output = Command::new("dotenvx")
            .args(["encrypt", "-f", &self.env_path])
            .output();
        match output {
            Ok(o) if o.status.success() => "✓ 암호화 완료".to_string(),
            Ok(o) => format!("✗ {}", String::from_utf8_lossy(&o.stderr).trim()),
            Err(e) => format!("✗ dotenvx: {}", e),
        }
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

        // Top bar: search or input mode
        let top_text = match &self.mode {
            Mode::Search => format!("/{}", self.search),
            Mode::AddKey { buf } => format!("[NEW KEY] {}", buf),
            Mode::AddValue { key: env_key, buf } => format!("[{}] = {}", env_key, buf),
            Mode::EditValue { idx, buf } => {
                let key = self.entries.get(*idx).map(|e| e.key.as_str()).unwrap_or("?");
                format!("[EDIT {}] = {}", key, buf)
            }
            Mode::Normal => {
                if self.search.is_empty() {
                    format!(" {} env vars", self.filtered.len())
                } else {
                    format!("/{} ({} results)", self.search, self.filtered.len())
                }
            }
        };
        let top_style = match &self.mode {
            Mode::Normal => Style::default().fg(Color::DarkGray),
            Mode::Search => Style::default().fg(Color::Cyan),
            Mode::AddKey { .. } | Mode::AddValue { .. } => Style::default().fg(Color::Green),
            Mode::EditValue { .. } => Style::default().fg(Color::Yellow),
        };
        frame.render_widget(
            Paragraph::new(top_text).block(
                Block::default().borders(Borders::ALL).border_style(top_style).title(" Env "),
            ),
            left[0],
        );

        // Table
        let visible_height = left[1].height.saturating_sub(2) as usize;
        let scroll = if self.selected >= visible_height { self.selected - visible_height + 1 } else { 0 };

        let rows: Vec<Row> = self.filtered.iter()
            .skip(scroll).take(visible_height).enumerate()
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
                    Cell::from(display_val).style(base.fg(if entry.encrypted && !self.show_decrypted { Color::DarkGray } else { Color::White })),
                ]).style(base)
            })
            .collect();

        let header = Row::new(vec!["", "Key", "Value"]).style(Style::default().fg(Color::Yellow).bold());
        let table = Table::new(rows, [
            Constraint::Length(4), Constraint::Percentage(30), Constraint::Percentage(60),
        ]).header(header).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" ~/.env ({}) {} ", self.entries.len(), if self.show_decrypted { "[DECRYPTED]" } else { "" })),
        );
        frame.render_widget(table, left[1]);

        // Right
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Length(14), Constraint::Min(0)])
            .split(chunks[1]);

        // Detail
        let detail_text = if let Some(entry) = self.selected_entry() {
            let val = if self.show_decrypted && entry.encrypted {
                self.cached_value(&entry.key).to_string()
            } else if entry.encrypted {
                format!("{}...", entry.value.chars().take(50).collect::<String>())
            } else {
                entry.value.clone()
            };
            format!("Key: {}\nEncrypted: {}\n\nValue:\n{}", entry.key, entry.encrypted, val)
        } else {
            "No entry selected".to_string()
        };
        frame.render_widget(
            Paragraph::new(detail_text).wrap(Wrap { trim: true }).block(
                Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Details "),
            ),
            right[0],
        );

        // Actions
        let actions = Paragraph::new(vec![
            Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            Line::from(vec![Span::styled("  a", Style::default().fg(Color::Green).bold()), Span::raw("  Add new key")]),
            Line::from(vec![Span::styled("  Enter", Style::default().fg(Color::Yellow).bold()), Span::raw("  Edit value")]),
            Line::from(vec![Span::styled("  x", Style::default().fg(Color::Red).bold()), Span::raw("  Delete key")]),
            Line::from(vec![Span::styled("  d", Style::default().fg(Color::Cyan).bold()), Span::raw("  Toggle decrypt")]),
            Line::from(vec![Span::styled("  e", Style::default().fg(Color::Cyan).bold()), Span::raw("  Re-encrypt all")]),
            Line::from(vec![Span::styled("  /", Style::default().fg(Color::Cyan).bold()), Span::raw("  Search")]),
            Line::from(vec![Span::styled("  r", Style::default().fg(Color::Cyan).bold()), Span::raw("  Refresh")]),
        ]).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Actions "),
        );
        frame.render_widget(actions, right[1]);

        // Output
        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true }).block(
                Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Output "),
            ),
            right[2],
        );
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            Mode::Search => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => self.mode = Mode::Normal,
                    KeyCode::Backspace => { self.search.pop(); self.apply_filter(); }
                    KeyCode::Char(c) => { self.search.push(c); self.apply_filter(); }
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddKey { buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        let k = buf.clone();
                        if k.is_empty() {
                            self.mode = Mode::Normal;
                        } else {
                            self.mode = Mode::AddValue { key: k, buf: String::new() };
                        }
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) => buf.push(c.to_ascii_uppercase()),
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddValue { key: env_key, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        let k = env_key.clone();
                        let v = buf.clone();
                        self.output = self.dotenvx_set(&k, &v);
                        self.mode = Mode::Normal;
                        self.show_decrypted = false;
                        self.decrypt_cache.clear();
                        self.load().await?;
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::EditValue { idx, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        let k = self.entries.get(*idx).map(|e| e.key.clone()).unwrap_or_default();
                        let v = buf.clone();
                        self.output = self.dotenvx_set(&k, &v);
                        self.mode = Mode::Normal;
                        self.show_decrypted = false;
                        self.decrypt_cache.clear();
                        self.load().await?;
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::Normal => {}
        }

        // Normal mode
        match key.code {
            KeyCode::Char('/') => self.mode = Mode::Search,
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.filtered.len() { self.selected += 1; }
            }
            KeyCode::Char('a') => {
                self.mode = Mode::AddKey { buf: String::new() };
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered.get(self.selected) {
                    let current = if self.show_decrypted {
                        self.cached_value(&self.entries[idx].key).to_string()
                    } else if self.entries[idx].encrypted {
                        String::new()
                    } else {
                        self.entries[idx].value.clone()
                    };
                    self.mode = Mode::EditValue { idx, buf: current };
                }
            }
            KeyCode::Char('x') => {
                if let Some(entry) = self.selected_entry() {
                    let k = entry.key.clone();
                    self.output = self.dotenvx_delete(&k);
                    self.show_decrypted = false;
                    self.decrypt_cache.clear();
                    self.load().await?;
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
                self.output = self.dotenvx_encrypt();
                self.show_decrypted = false;
                self.decrypt_cache.clear();
                self.load().await?;
            }
            _ => {}
        }
        Ok(())
    }
}
