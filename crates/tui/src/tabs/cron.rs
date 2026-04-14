use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use mac_host_core::cron;
use mac_host_core::models::cron::LaunchAgent;
use std::fs;
use std::process::Command;

enum Mode {
    Normal,
    // Add form steps
    AddLabel { buf: String },
    AddProgram { label: String, buf: String },
    AddScheduleType { label: String, program: String, selected: usize },
    AddCalendarHour { label: String, program: String, buf: String },
    AddCalendarMinute { label: String, program: String, hour: String, buf: String },
    AddInterval { label: String, program: String, buf: String },
    AddWatchPath { label: String, program: String, buf: String },
    // Edit
    EditProgram { idx: usize, buf: String },
}

const SCHEDULE_TYPES: &[&str] = &["Calendar (시간 지정)", "Interval (반복 간격)", "WatchPaths (파일 감시)", "RunAtLoad (시작 시 실행)"];

pub struct CronTab {
    agents: Vec<LaunchAgent>,
    selected: usize,
    output: String,
    mode: Mode,
}

impl CronTab {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected: 0,
            output: String::new(),
            mode: Mode::Normal,
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.agents = cron::get_agents();
        Ok(())
    }

    fn home() -> String {
        std::env::var("HOME").unwrap_or_default()
    }

    fn create_plist(label: &str, program: &str, schedule_xml: &str) -> std::result::Result<String, String> {
        let home = Self::home();
        let plist_path = format!("{}/Library/LaunchAgents/{}.plist", home, label);

        let content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-c</string>
        <string>{program}</string>
    </array>
{schedule_xml}
    <key>StandardOutPath</key>
    <string>{home}/문서/시스템/로그/{label}.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/문서/시스템/로그/{label}.log</string>
</dict>
</plist>"#);

        // Ensure log dir
        let log_dir = format!("{}/문서/시스템/로그", home);
        fs::create_dir_all(&log_dir).ok();

        fs::write(&plist_path, content).map_err(|e| format!("plist 생성 실패: {}", e))?;

        // Load
        let _ = Command::new("launchctl").args(["unload", &plist_path]).output();
        let output = Command::new("launchctl").args(["load", &plist_path]).output();
        match output {
            Ok(o) if o.status.success() => Ok(format!("✓ {} 등록 + 로드 완료", label)),
            Ok(o) => Ok(format!("⚠ plist 생성됨, 로드 실패: {}", String::from_utf8_lossy(&o.stderr).trim())),
            Err(e) => Ok(format!("⚠ plist 생성됨, 로드 실패: {}", e)),
        }
    }

    fn delete_agent(agent: &LaunchAgent) -> String {
        let _ = Command::new("launchctl").args(["unload", &agent.path.to_string_lossy()]).output();
        match fs::remove_file(&agent.path) {
            Ok(_) => format!("✓ {} 삭제 완료", agent.label),
            Err(e) => format!("✗ 삭제 실패: {}", e),
        }
    }

    fn schedule_xml_calendar(hour: &str, minute: &str) -> String {
        let h: u32 = hour.parse().unwrap_or(9);
        let m: u32 = minute.parse().unwrap_or(0);
        format!(r#"    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>{h}</integer>
        <key>Minute</key>
        <integer>{m}</integer>
    </dict>"#)
    }

    fn schedule_xml_interval(seconds: &str) -> String {
        let s: u32 = seconds.parse().unwrap_or(3600);
        format!(r#"    <key>StartInterval</key>
    <integer>{s}</integer>"#)
    }

    fn schedule_xml_watch(path: &str) -> String {
        format!(r#"    <key>WatchPaths</key>
    <array>
        <string>{path}</string>
    </array>"#)
    }

    fn schedule_xml_run_at_load() -> String {
        r#"    <key>RunAtLoad</key>
    <true/>"#.to_string()
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

        // Top bar
        let top_text = match &self.mode {
            Mode::Normal => format!(" {} agents", self.agents.len()),
            Mode::AddLabel { buf } => format!("[Label] {}", buf),
            Mode::AddProgram { label, buf } => format!("[{}] Program: {}", label, buf),
            Mode::AddScheduleType { label, .. } => format!("[{}] Schedule type:", label),
            Mode::AddCalendarHour { label, buf, .. } => format!("[{}] Hour (0-23): {}", label, buf),
            Mode::AddCalendarMinute { label, hour, buf, .. } => format!("[{}] {}:{} Minute (0-59): {}", label, hour, buf, buf),
            Mode::AddInterval { label, buf, .. } => format!("[{}] Interval (초): {}", label, buf),
            Mode::AddWatchPath { label, buf, .. } => format!("[{}] Watch path: {}", label, buf),
            Mode::EditProgram { idx, buf } => {
                let label = self.agents.get(*idx).map(|a| a.label.as_str()).unwrap_or("?");
                format!("[EDIT {}] Program: {}", label, buf)
            }
        };
        let top_style = match &self.mode {
            Mode::Normal => Style::default().fg(Color::DarkGray),
            Mode::AddScheduleType { .. } => Style::default().fg(Color::Magenta),
            _ => Style::default().fg(Color::Green),
        };
        frame.render_widget(
            Paragraph::new(top_text).block(
                Block::default().borders(Borders::ALL).border_style(top_style).title(" Cron "),
            ),
            left[0],
        );

        // Schedule type selector (when in that mode)
        if let Mode::AddScheduleType { selected, .. } = &self.mode {
            let items: Vec<ListItem> = SCHEDULE_TYPES.iter().enumerate().map(|(i, t)| {
                let style = if i == *selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White).bold()
                } else {
                    Style::default()
                };
                ListItem::new(format!("  {} ", t)).style(style)
            }).collect();
            let list = List::new(items).block(
                Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta))
                    .title(" Schedule Type (Enter to select) "),
            );
            frame.render_widget(list, left[1]);
        } else {
            // Agent table
            let visible_height = left[1].height.saturating_sub(2) as usize;
            let scroll = self.selected.saturating_sub(visible_height.saturating_sub(1));

            let rows: Vec<Row> = self.agents.iter()
                .skip(scroll).take(visible_height).enumerate()
                .map(|(vis_i, agent)| {
                    let is_sel = scroll + vis_i == self.selected;
                    let base = if is_sel { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() };
                    let status_style = if agent.running { Style::default().fg(Color::Green) }
                        else if agent.loaded { Style::default().fg(Color::Yellow) }
                        else { Style::default().fg(Color::Red) };
                    let status = if agent.running { format!("run({})", agent.pid.unwrap_or(0)) }
                        else if agent.loaded { "loaded".into() } else { "stop".into() };
                    Row::new(vec![
                        Cell::from(status).style(if is_sel { base } else { status_style }),
                        Cell::from(agent.label.as_str()).style(base),
                        Cell::from(agent.schedule.as_str()).style(base),
                    ])
                }).collect();

            let header = Row::new(vec!["Status", "Label", "Schedule"]).style(Style::default().fg(Color::Yellow).bold());
            let table = Table::new(rows, [
                Constraint::Length(10), Constraint::Min(20), Constraint::Length(25),
            ]).header(header).block(
                Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" LaunchAgents ({}) ", self.agents.len())),
            );
            frame.render_widget(table, left[1]);
        }

        // Right
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Length(14), Constraint::Min(0)])
            .split(chunks[1]);

        // Detail
        let detail = if let Some(agent) = self.agents.get(self.selected) {
            format!(
                "Label: {}\nStatus: {}\nSchedule: {}\nProgram: {}\nPath: {}",
                agent.label,
                if agent.running { format!("Running (PID {})", agent.pid.unwrap_or(0)) }
                else if agent.loaded { "Loaded".into() } else { "Stopped".into() },
                agent.schedule, agent.program, agent.path.display(),
            )
        } else { String::new() };
        frame.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: true }).block(
                Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Details "),
            ), right[0],
        );

        // Actions
        let actions = Paragraph::new(vec![
            Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            Line::from(vec![Span::styled("  a", Style::default().fg(Color::Green).bold()), Span::raw("  Add new agent")]),
            Line::from(vec![Span::styled("  x", Style::default().fg(Color::Red).bold()), Span::raw("  Delete agent")]),
            Line::from(vec![Span::styled("  Enter", Style::default().fg(Color::Yellow).bold()), Span::raw("  Edit program")]),
            Line::from(vec![Span::styled("  l", Style::default().fg(Color::Cyan).bold()), Span::raw("  Load (start)")]),
            Line::from(vec![Span::styled("  s", Style::default().fg(Color::Cyan).bold()), Span::raw("  Stop (unload)")]),
            Line::from(vec![Span::styled("  R", Style::default().fg(Color::Cyan).bold()), Span::raw("  Restart")]),
            Line::from(vec![Span::styled("  r", Style::default().fg(Color::Cyan).bold()), Span::raw("  Refresh")]),
        ]).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Actions "),
        );
        frame.render_widget(actions, right[1]);

        // Output
        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true }).block(
                Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Output "),
            ), right[2],
        );
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            Mode::AddLabel { buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter if !buf.is_empty() => {
                        let label = buf.clone();
                        self.mode = Mode::AddProgram { label, buf: String::new() };
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddProgram { label, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter if !buf.is_empty() => {
                        let l = label.clone();
                        let p = buf.clone();
                        self.mode = Mode::AddScheduleType { label: l, program: p, selected: 0 };
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddScheduleType { label, program, selected } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Up | KeyCode::Char('k') => *selected = selected.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => {
                        if *selected + 1 < SCHEDULE_TYPES.len() { *selected += 1; }
                    }
                    KeyCode::Enter => {
                        let l = label.clone();
                        let p = program.clone();
                        match *selected {
                            0 => self.mode = Mode::AddCalendarHour { label: l, program: p, buf: String::new() },
                            1 => self.mode = Mode::AddInterval { label: l, program: p, buf: String::new() },
                            2 => self.mode = Mode::AddWatchPath { label: l, program: p, buf: String::new() },
                            3 => {
                                let xml = Self::schedule_xml_run_at_load();
                                self.output = Self::create_plist(&l, &p, &xml).unwrap_or_else(|e| e);
                                self.mode = Mode::Normal;
                                self.load().await?;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddCalendarHour { label, program, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        let l = label.clone();
                        let p = program.clone();
                        let h = buf.clone();
                        self.mode = Mode::AddCalendarMinute { label: l, program: p, hour: h, buf: "0".into() };
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) if c.is_ascii_digit() => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddCalendarMinute { label, program, hour, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        let xml = Self::schedule_xml_calendar(hour, buf);
                        self.output = Self::create_plist(label, program, &xml).unwrap_or_else(|e| e);
                        self.mode = Mode::Normal;
                        self.load().await?;
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) if c.is_ascii_digit() => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddInterval { label, program, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        let xml = Self::schedule_xml_interval(buf);
                        self.output = Self::create_plist(label, program, &xml).unwrap_or_else(|e| e);
                        self.mode = Mode::Normal;
                        self.load().await?;
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) if c.is_ascii_digit() => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::AddWatchPath { label, program, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        let xml = Self::schedule_xml_watch(buf);
                        self.output = Self::create_plist(label, program, &xml).unwrap_or_else(|e| e);
                        self.mode = Mode::Normal;
                        self.load().await?;
                    }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
                return Ok(());
            }
            Mode::EditProgram { idx, buf } => {
                match key.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        if let Some(agent) = self.agents.get(*idx) {
                            // Re-create plist with new program, keep schedule
                            let plist_content = fs::read_to_string(&agent.path).unwrap_or_default();
                            let new_content = plist_content.replace(&agent.program, buf);
                            if let Err(e) = fs::write(&agent.path, new_content) {
                                self.output = format!("✗ 수정 실패: {}", e);
                            } else {
                                let _ = Command::new("launchctl").args(["unload", &agent.path.to_string_lossy()]).output();
                                let _ = Command::new("launchctl").args(["load", &agent.path.to_string_lossy()]).output();
                                self.output = format!("✓ {} 프로그램 수정 + 재로드", agent.label);
                            }
                        }
                        self.mode = Mode::Normal;
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
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.agents.len() { self.selected += 1; }
            }
            KeyCode::Char('a') => {
                self.mode = Mode::AddLabel { buf: "com.mac-app-init.".into() };
            }
            KeyCode::Enter => {
                if let Some(agent) = self.agents.get(self.selected) {
                    self.mode = Mode::EditProgram { idx: self.selected, buf: agent.program.clone() };
                }
            }
            KeyCode::Char('x') => {
                if let Some(agent) = self.agents.get(self.selected) {
                    self.output = Self::delete_agent(agent);
                    self.load().await?;
                }
            }
            KeyCode::Char('l') => {
                if let Some(agent) = self.agents.get(self.selected) {
                    match cron::load_agent(&agent.label) {
                        Ok(msg) => self.output = msg,
                        Err(msg) => self.output = msg,
                    }
                    self.load().await?;
                }
            }
            KeyCode::Char('s') => {
                if let Some(agent) = self.agents.get(self.selected) {
                    match cron::unload_agent(&agent.label) {
                        Ok(msg) => self.output = msg,
                        Err(msg) => self.output = msg,
                    }
                    self.load().await?;
                }
            }
            KeyCode::Char('R') => {
                if let Some(agent) = self.agents.get(self.selected) {
                    match cron::restart_agent(&agent.label) {
                        Ok(msg) => self.output = msg,
                        Err(msg) => self.output = msg,
                    }
                    self.load().await?;
                }
            }
            KeyCode::Char('r') => {
                self.load().await?;
                self.output = "Refreshed.".into();
            }
            _ => {}
        }
        Ok(())
    }
}
