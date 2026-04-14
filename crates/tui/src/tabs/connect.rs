use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Connection {
    name: String,
    host: String,
    user: String,
    port: u16,
    #[serde(default)]
    extra: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Connections {
    services: Vec<Connection>,
}

enum Mode {
    Normal,
    AddName { buf: String },
    AddHost { name: String, buf: String },
    AddUser { name: String, host: String, buf: String },
    AddPort { name: String, host: String, user: String, buf: String },
    AddExtra { name: String, host: String, user: String, port: u16, key_buf: String, val_buf: String, entering_key: bool, extras: HashMap<String, String> },
    TestingAll,
}

pub struct ConnectTab {
    connections: Vec<Connection>,
    test_results: HashMap<String, bool>,
    selected: usize,
    output: String,
    mode: Mode,
}

fn connections_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".mac-app-init/connections.json")
}

fn load_connections() -> Vec<Connection> {
    let path = connections_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let c: Connections = serde_json::from_str(&content).unwrap_or_default();
        c.services
    } else {
        Vec::new()
    }
}

fn save_connections(conns: &[Connection]) {
    let c = Connections { services: conns.to_vec() };
    let path = connections_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    fs::write(&path, serde_json::to_string_pretty(&c).unwrap()).ok();
}

fn env_key(service: &str, field: &str) -> String {
    format!("{}_{}", service.to_uppercase().replace('-', "_"), field.to_uppercase())
}

fn dotenvx_set(key: &str, value: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let env_path = format!("{}/.env", home);
    let output = Command::new("dotenvx")
        .args(["set", key, value, "-f", &env_path])
        .output();
    match output {
        Ok(o) if o.status.success() => format!("✓ {} 저장", key),
        _ => format!("✗ {} 저장 실패", key),
    }
}

fn test_connection(conn: &Connection) -> bool {
    Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=3", "-p", &conn.port.to_string(), &format!("{}@{}", conn.user, conn.host), "echo ok"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or_else(|_| {
            Command::new("ping").args(["-c", "1", "-W", "3", &conn.host]).output().map(|o| o.status.success()).unwrap_or(false)
        })
}

impl ConnectTab {
    pub fn new() -> Self {
        Self { connections: Vec::new(), test_results: HashMap::new(), selected: 0, output: String::new(), mode: Mode::Normal }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.connections = load_connections();
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(chunks[0]);

        // Top bar
        let top = match &self.mode {
            Mode::Normal => format!(" {} connections", self.connections.len()),
            Mode::AddName { buf } => format!("[Name] {}", buf),
            Mode::AddHost { name, buf } => format!("[{}] Host: {}", name, buf),
            Mode::AddUser { name, buf, .. } => format!("[{}] User: {}", name, buf),
            Mode::AddPort { name, buf, .. } => format!("[{}] Port: {}", name, buf),
            Mode::AddExtra { name, entering_key, key_buf, val_buf, .. } => {
                if *entering_key {
                    format!("[{}] Extra key (빈값=완료): {}", name, key_buf)
                } else {
                    format!("[{}] {} = {}", name, key_buf, val_buf)
                }
            }
            Mode::TestingAll => "Testing all connections...".into(),
        };
        let top_style = match &self.mode {
            Mode::Normal | Mode::TestingAll => Style::default().fg(Color::DarkGray),
            _ => Style::default().fg(Color::Green),
        };
        frame.render_widget(
            Paragraph::new(top).block(Block::default().borders(Borders::ALL).border_style(top_style).title(" Connect ")),
            left[0],
        );

        // Table
        let vis = left[1].height.saturating_sub(2) as usize;
        let scroll = self.selected.saturating_sub(vis.saturating_sub(1));
        let rows: Vec<Row> = self.connections.iter().skip(scroll).take(vis).enumerate().map(|(vi, c)| {
            let sel = scroll + vi == self.selected;
            let base = if sel { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() };
            let test_ok = self.test_results.get(&c.name);
            let status = match test_ok {
                Some(true) => "✓",
                Some(false) => "✗",
                None => "?",
            };
            let st = match test_ok {
                Some(true) => Style::default().fg(Color::Green),
                Some(false) => Style::default().fg(Color::Red),
                None => Style::default().fg(Color::DarkGray),
            };
            Row::new(vec![
                Cell::from(status).style(if sel { base } else { st }),
                Cell::from(c.name.as_str()).style(base),
                Cell::from(format!("{}@{}:{}", c.user, c.host, c.port)).style(base),
            ])
        }).collect();

        let header = Row::new(vec!["", "Name", "Connection"]).style(Style::default().fg(Color::Yellow).bold());
        frame.render_widget(
            Table::new(rows, [Constraint::Length(2), Constraint::Length(15), Constraint::Min(20)])
                .header(header).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))
                .title(" Connections ")),
            left[1],
        );

        // Right
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Length(13), Constraint::Min(0)])
            .split(chunks[1]);

        // Detail
        let detail = self.connections.get(self.selected).map(|c| {
            let mut s = format!("Name: {}\nHost: {}\nUser: {}\nPort: {}", c.name, c.host, c.user, c.port);
            if !c.extra.is_empty() {
                s.push_str("\n\nExtra:");
                for (k, _) in &c.extra {
                    s.push_str(&format!("\n  {} = ****", k));
                }
            }
            let test = self.test_results.get(&c.name);
            s.push_str(&format!("\n\nStatus: {}", match test {
                Some(true) => "Connected",
                Some(false) => "Failed",
                None => "Not tested",
            }));
            s
        }).unwrap_or_else(|| "No connections.\n  a: add new".into());
        frame.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Details ")),
            right[0],
        );

        // Actions
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
                Line::from(""),
                Line::from(vec![Span::styled("  a", Style::default().fg(Color::Green).bold()), Span::raw("  Add connection")]),
                Line::from(vec![Span::styled("  x", Style::default().fg(Color::Red).bold()), Span::raw("  Delete connection")]),
                Line::from(vec![Span::styled("  t", Style::default().fg(Color::Cyan).bold()), Span::raw("  Test selected")]),
                Line::from(vec![Span::styled("  T", Style::default().fg(Color::Cyan).bold()), Span::raw("  Test all")]),
                Line::from(vec![Span::styled("  e", Style::default().fg(Color::Cyan).bold()), Span::raw("  Re-encrypt .env")]),
                Line::from(vec![Span::styled("  r", Style::default().fg(Color::Cyan).bold()), Span::raw("  Refresh")]),
            ]).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Actions ")),
            right[1],
        );

        // Output
        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Output ")),
            right[2],
        );
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            Mode::AddName { buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter if !buf.is_empty() => { let n = buf.clone(); self.mode = Mode::AddHost { name: n, buf: String::new() }; }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddHost { name, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter if !buf.is_empty() => { let n = name.clone(); let h = buf.clone(); self.mode = Mode::AddUser { name: n, host: h, buf: "root".into() }; }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddUser { name, host, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => { let n = name.clone(); let h = host.clone(); let u = if buf.is_empty() { "root".into() } else { buf.clone() }; self.mode = Mode::AddPort { name: n, host: h, user: u, buf: "22".into() }; }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddPort { name, host, user, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    let n = name.clone(); let h = host.clone(); let u = user.clone();
                    let p: u16 = buf.parse().unwrap_or(22);
                    self.mode = Mode::AddExtra { name: n, host: h, user: u, port: p, key_buf: String::new(), val_buf: String::new(), entering_key: true, extras: HashMap::new() };
                }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) if c.is_ascii_digit() => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddExtra { name, host, user, port, key_buf, val_buf, entering_key, extras } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    if *entering_key {
                        if key_buf.is_empty() {
                            // Done adding extras, save
                            let conn = Connection { name: name.clone(), host: host.clone(), user: user.clone(), port: *port, extra: extras.clone() };
                            self.connections.push(conn);
                            save_connections(&self.connections);
                            // Save to .env
                            let mut out = Vec::new();
                            out.push(dotenvx_set(&env_key(name, "HOST"), host));
                            out.push(dotenvx_set(&env_key(name, "USER"), user));
                            out.push(dotenvx_set(&env_key(name, "PORT"), &port.to_string()));
                            for (k, v) in extras.iter() {
                                out.push(dotenvx_set(&env_key(name, k), v));
                            }
                            self.output = out.join("\n");
                            self.mode = Mode::Normal;
                        } else {
                            *entering_key = false;
                        }
                    } else {
                        if !val_buf.is_empty() {
                            extras.insert(key_buf.clone(), val_buf.clone());
                        }
                        *key_buf = String::new();
                        *val_buf = String::new();
                        *entering_key = true;
                    }
                }
                KeyCode::Backspace => {
                    if *entering_key { key_buf.pop(); } else { val_buf.pop(); }
                }
                KeyCode::Char(c) => {
                    if *entering_key { key_buf.push(c.to_ascii_uppercase()); } else { val_buf.push(c); }
                }
                _ => {}
            } return Ok(()); }
            Mode::TestingAll | Mode::Normal => {}
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => { if self.selected + 1 < self.connections.len() { self.selected += 1; } }
            KeyCode::Char('a') => self.mode = Mode::AddName { buf: String::new() },
            KeyCode::Char('x') => {
                if let Some(c) = self.connections.get(self.selected) {
                    let name = c.name.clone();
                    self.connections.retain(|c| c.name != name);
                    save_connections(&self.connections);
                    self.test_results.remove(&name);
                    self.output = format!("✓ {} 삭제", name);
                    if self.selected >= self.connections.len() { self.selected = self.connections.len().saturating_sub(1); }
                }
            }
            KeyCode::Char('t') => {
                if let Some(c) = self.connections.get(self.selected) {
                    self.output = format!("Testing {}...", c.name);
                    let ok = test_connection(c);
                    self.test_results.insert(c.name.clone(), ok);
                    self.output = format!("{} {}", c.name, if ok { "✓ Connected" } else { "✗ Failed" });
                }
            }
            KeyCode::Char('T') => {
                self.output = "Testing all...\n".into();
                for c in &self.connections {
                    let ok = test_connection(c);
                    self.test_results.insert(c.name.clone(), ok);
                    self.output.push_str(&format!("  {} {} {}\n", if ok { "✓" } else { "✗" }, c.name, c.host));
                }
            }
            KeyCode::Char('e') => {
                let home = std::env::var("HOME").unwrap_or_default();
                let out = Command::new("dotenvx").args(["encrypt", "-f", &format!("{}/.env", home)]).output();
                self.output = match out {
                    Ok(o) if o.status.success() => "✓ .env 암호화 완료".into(),
                    _ => "✗ 암호화 실패".into(),
                };
            }
            KeyCode::Char('r') => { self.load().await?; self.output = "Refreshed.".into(); }
            _ => {}
        }
        Ok(())
    }
}
