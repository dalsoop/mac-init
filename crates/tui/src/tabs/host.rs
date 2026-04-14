use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::fs;
use std::process::Command;

struct HostEntry {
    ip: String,
    hostname: String,
    comment: bool,
    raw: String,
}

enum Mode {
    Normal,
    AddIp { buf: String },
    AddHostname { ip: String, buf: String },
}

pub struct HostTab {
    entries: Vec<HostEntry>,
    selected: usize,
    output: String,
    mode: Mode,
}

const HOSTS_PATH: &str = "/etc/hosts";

fn parse_hosts() -> Vec<HostEntry> {
    let content = fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    let mut entries = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("##") || trimmed.starts_with("# =") {
            continue;
        }

        let (comment, effective) = if let Some(stripped) = trimmed.strip_prefix('#') {
            (true, stripped.trim())
        } else {
            (false, trimmed)
        };

        let parts: Vec<&str> = effective.split_whitespace().collect();
        if parts.len() >= 2 {
            entries.push(HostEntry {
                ip: parts[0].to_string(),
                hostname: parts[1..].join(" "),
                comment,
                raw: line.to_string(),
            });
        }
    }
    entries
}

fn save_entry(ip: &str, hostname: &str) -> String {
    let line = format!("{}\t{}", ip, hostname);
    let cmd = format!("echo '{}' | sudo tee -a /etc/hosts > /dev/null", line);
    match Command::new("bash").args(["-c", &cmd]).output() {
        Ok(o) if o.status.success() => format!("✓ {} → {} 추가", ip, hostname),
        _ => "✗ 추가 실패 (sudo 권한 필요)".into(),
    }
}

fn toggle_comment(entry: &HostEntry) -> String {
    let content = fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    let new_line = if entry.comment {
        entry.raw.strip_prefix('#').unwrap_or(&entry.raw).trim().to_string()
    } else {
        format!("#{}", entry.raw)
    };
    let new_content = content.replace(&entry.raw, &new_line);
    let cmd = format!("echo '{}' | sudo tee /etc/hosts > /dev/null", new_content.replace('\'', "'\\''"));
    match Command::new("bash").args(["-c", &cmd]).output() {
        Ok(o) if o.status.success() => format!("✓ {} {}", entry.hostname, if entry.comment { "활성화" } else { "비활성화" }),
        _ => "✗ 변경 실패 (sudo 권한 필요)".into(),
    }
}

fn delete_entry(entry: &HostEntry) -> String {
    let content = fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    let new_content: Vec<&str> = content.lines().filter(|l| *l != entry.raw).collect();
    let cmd = format!("echo '{}' | sudo tee /etc/hosts > /dev/null", new_content.join("\n").replace('\'', "'\\''"));
    match Command::new("bash").args(["-c", &cmd]).output() {
        Ok(o) if o.status.success() => format!("✓ {} 삭제", entry.hostname),
        _ => "✗ 삭제 실패 (sudo 권한 필요)".into(),
    }
}

impl HostTab {
    pub fn new() -> Self {
        Self { entries: Vec::new(), selected: 0, output: String::new(), mode: Mode::Normal }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.entries = parse_hosts();
        Ok(())
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

        let top = match &self.mode {
            Mode::Normal => format!(" /etc/hosts ({} entries)", self.entries.len()),
            Mode::AddIp { buf } => format!("[IP] {}", buf),
            Mode::AddHostname { ip, buf } => format!("[{}] Hostname: {}", ip, buf),
        };
        let top_style = match &self.mode {
            Mode::Normal => Style::default().fg(Color::DarkGray),
            _ => Style::default().fg(Color::Green),
        };
        frame.render_widget(
            Paragraph::new(top).block(Block::default().borders(Borders::ALL).border_style(top_style).title(" Hosts ")),
            left[0],
        );

        let vis = left[1].height.saturating_sub(2) as usize;
        let scroll = self.selected.saturating_sub(vis.saturating_sub(1));
        let rows: Vec<Row> = self.entries.iter().skip(scroll).take(vis).enumerate().map(|(vi, e)| {
            let sel = scroll + vi == self.selected;
            let base = if sel { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() };
            let st = if e.comment { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::Green) };
            Row::new(vec![
                Cell::from(if e.comment { "#" } else { "✓" }).style(if sel { base } else { st }),
                Cell::from(e.ip.as_str()).style(base),
                Cell::from(e.hostname.as_str()).style(base),
            ])
        }).collect();

        let header = Row::new(vec!["", "IP", "Hostname"]).style(Style::default().fg(Color::Yellow).bold());
        frame.render_widget(
            Table::new(rows, [Constraint::Length(2), Constraint::Length(20), Constraint::Min(20)])
                .header(header).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" /etc/hosts ")),
            left[1],
        );

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Length(11), Constraint::Min(0)])
            .split(chunks[1]);

        let detail = self.entries.get(self.selected).map(|e| {
            format!("IP: {}\nHostname: {}\nActive: {}", e.ip, e.hostname, !e.comment)
        }).unwrap_or_default();
        frame.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Details ")),
            right[0],
        );

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
                Line::from(""),
                Line::from(vec![Span::styled("  a", Style::default().fg(Color::Green).bold()), Span::raw("  Add entry")]),
                Line::from(vec![Span::styled("  x", Style::default().fg(Color::Red).bold()), Span::raw("  Delete entry")]),
                Line::from(vec![Span::styled("  t", Style::default().fg(Color::Cyan).bold()), Span::raw("  Toggle on/off")]),
                Line::from(vec![Span::styled("  r", Style::default().fg(Color::Cyan).bold()), Span::raw("  Refresh")]),
                Line::from(Span::styled("  ⚠ sudo 필요", Style::default().fg(Color::Yellow))),
            ]).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Actions ")),
            right[1],
        );

        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Output ")),
            right[2],
        );
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            Mode::AddIp { buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter if !buf.is_empty() => { let ip = buf.clone(); self.mode = Mode::AddHostname { ip, buf: String::new() }; }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddHostname { ip, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter if !buf.is_empty() => {
                    self.output = save_entry(ip, buf);
                    self.mode = Mode::Normal;
                    self.load().await?;
                }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::Normal => {}
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => { if self.selected + 1 < self.entries.len() { self.selected += 1; } }
            KeyCode::Char('a') => self.mode = Mode::AddIp { buf: String::new() },
            KeyCode::Char('x') => {
                if let Some(e) = self.entries.get(self.selected) {
                    self.output = delete_entry(e);
                    self.load().await?;
                }
            }
            KeyCode::Char('t') => {
                if let Some(e) = self.entries.get(self.selected) {
                    self.output = toggle_comment(e);
                    self.load().await?;
                }
            }
            KeyCode::Char('r') => { self.load().await?; self.output = "Refreshed.".into(); }
            _ => {}
        }
        Ok(())
    }
}
