use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    name: String,
    command: String,
    schedule: Schedule,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    description: String,
}
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Schedule {
    #[serde(rename = "type")]
    stype: String,
    #[serde(default)]
    cron: Option<String>,
    #[serde(default)]
    interval_seconds: Option<u64>,
    #[serde(default)]
    watch_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScheduleFile {
    jobs: Vec<Job>,
}

enum Mode {
    Normal,
    AddName { buf: String },
    AddCommand { name: String, buf: String },
    AddScheduleType { name: String, command: String, selected: usize },
    AddCron { name: String, command: String, buf: String },
    AddInterval { name: String, command: String, buf: String },
    AddWatch { name: String, command: String, buf: String },
}

const SCHED_TYPES: &[&str] = &["Cron (분 시 일 월 요일)", "Interval (초)", "Watch (경로)"];

pub struct CronTab {
    jobs: Vec<Job>,
    selected: usize,
    output: String,
    mode: Mode,
}

fn schedule_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".mac-app-init/schedule.json")
}

fn load_jobs() -> Vec<Job> {
    let path = schedule_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let sf: ScheduleFile = serde_json::from_str(&content).unwrap_or(ScheduleFile { jobs: vec![] });
        sf.jobs
    } else {
        Vec::new()
    }
}

fn save_jobs(jobs: &[Job]) {
    let sf = ScheduleFile { jobs: jobs.to_vec() };
    let path = schedule_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    let json = serde_json::to_string_pretty(&sf).unwrap();
    fs::write(&path, json).ok();
}

impl CronTab {
    pub fn new() -> Self {
        Self { jobs: Vec::new(), selected: 0, output: String::new(), mode: Mode::Normal }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.jobs = load_jobs();
        Ok(())
    }

    fn schedule_display(job: &Job) -> String {
        match job.schedule.stype.as_str() {
            "cron" => job.schedule.cron.clone().unwrap_or_default(),
            "interval" => format!("every {}s", job.schedule.interval_seconds.unwrap_or(0)),
            "watch" => format!("watch:{}", job.schedule.watch_path.clone().unwrap_or_default()),
            _ => "?".into(),
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

        // Top bar
        let top = match &self.mode {
            Mode::Normal => format!(" {} jobs (schedule.json)", self.jobs.len()),
            Mode::AddName { buf } => format!("[Name] {}", buf),
            Mode::AddCommand { name, buf } => format!("[{}] Command: {}", name, buf),
            Mode::AddScheduleType { name, .. } => format!("[{}] Schedule:", name),
            Mode::AddCron { name, buf, .. } => format!("[{}] Cron: {}", name, buf),
            Mode::AddInterval { name, buf, .. } => format!("[{}] Interval(초): {}", name, buf),
            Mode::AddWatch { name, buf, .. } => format!("[{}] Watch: {}", name, buf),
        };
        let top_style = match &self.mode {
            Mode::Normal => Style::default().fg(Color::DarkGray),
            Mode::AddScheduleType { .. } => Style::default().fg(Color::Magenta),
            _ => Style::default().fg(Color::Green),
        };
        frame.render_widget(
            Paragraph::new(top).block(Block::default().borders(Borders::ALL).border_style(top_style).title(" Scheduler ")),
            left[0],
        );

        if let Mode::AddScheduleType { selected, .. } = &self.mode {
            let items: Vec<ListItem> = SCHED_TYPES.iter().enumerate().map(|(i, t)| {
                let s = if i == *selected { Style::default().bg(Color::DarkGray).fg(Color::White).bold() } else { Style::default() };
                ListItem::new(format!("  {} ", t)).style(s)
            }).collect();
            frame.render_widget(
                List::new(items).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta)).title(" Type ")),
                left[1],
            );
        } else {
            let vis = left[1].height.saturating_sub(2) as usize;
            let scroll = self.selected.saturating_sub(vis.saturating_sub(1));
            let rows: Vec<Row> = self.jobs.iter().skip(scroll).take(vis).enumerate().map(|(vi, j)| {
                let sel = scroll + vi == self.selected;
                let base = if sel { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() };
                let st = if j.enabled { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };
                Row::new(vec![
                    Cell::from(if j.enabled { "✓" } else { "✗" }).style(if sel { base } else { st }),
                    Cell::from(j.name.as_str()).style(base),
                    Cell::from(Self::schedule_display(j)).style(base),
                ])
            }).collect();
            let header = Row::new(vec!["", "Name", "Schedule"]).style(Style::default().fg(Color::Yellow).bold());
            frame.render_widget(
                Table::new(rows, [Constraint::Length(2), Constraint::Min(15), Constraint::Length(25)])
                    .header(header).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" Jobs ({}) ", self.jobs.len()))),
                left[1],
            );
        }

        // Right
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Length(12), Constraint::Min(0)])
            .split(chunks[1]);

        let detail = self.jobs.get(self.selected).map(|j| {
            format!("Name: {}\nCommand: {}\nSchedule: {}\nEnabled: {}\n{}", j.name, j.command, Self::schedule_display(j), j.enabled, j.description)
        }).unwrap_or_default();
        frame.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Details ")),
            right[0],
        );

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
                Line::from(""),
                Line::from(vec![Span::styled("  a", Style::default().fg(Color::Green).bold()), Span::raw("  Add job")]),
                Line::from(vec![Span::styled("  x", Style::default().fg(Color::Red).bold()), Span::raw("  Delete job")]),
                Line::from(vec![Span::styled("  t", Style::default().fg(Color::Cyan).bold()), Span::raw("  Toggle enable/disable")]),
                Line::from(vec![Span::styled("  Enter", Style::default().fg(Color::Yellow).bold()), Span::raw("  Run now")]),
                Line::from(vec![Span::styled("  r", Style::default().fg(Color::Cyan).bold()), Span::raw("  Refresh")]),
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
            Mode::AddName { buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter if !buf.is_empty() => { let n = buf.clone(); self.mode = Mode::AddCommand { name: n, buf: String::new() }; }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddCommand { name, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter if !buf.is_empty() => { let n = name.clone(); let c = buf.clone(); self.mode = Mode::AddScheduleType { name: n, command: c, selected: 0 }; }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddScheduleType { name, command, selected } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Up | KeyCode::Char('k') => *selected = selected.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => { if *selected + 1 < SCHED_TYPES.len() { *selected += 1; } }
                KeyCode::Enter => {
                    let n = name.clone(); let c = command.clone();
                    match *selected {
                        0 => self.mode = Mode::AddCron { name: n, command: c, buf: "0 9 * * *".into() },
                        1 => self.mode = Mode::AddInterval { name: n, command: c, buf: "3600".into() },
                        2 => self.mode = Mode::AddWatch { name: n, command: c, buf: String::new() },
                        _ => {}
                    }
                }
                _ => {}
            } return Ok(()); }
            Mode::AddCron { name, command, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    self.jobs.push(Job { name: name.clone(), command: command.clone(), schedule: Schedule { stype: "cron".into(), cron: Some(buf.clone()), interval_seconds: None, watch_path: None }, enabled: true, description: String::new() });
                    save_jobs(&self.jobs); self.output = format!("✓ {} 추가 완료", name); self.mode = Mode::Normal;
                }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddInterval { name, command, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    let secs = buf.parse().unwrap_or(3600);
                    self.jobs.push(Job { name: name.clone(), command: command.clone(), schedule: Schedule { stype: "interval".into(), cron: None, interval_seconds: Some(secs), watch_path: None }, enabled: true, description: String::new() });
                    save_jobs(&self.jobs); self.output = format!("✓ {} 추가 완료", name); self.mode = Mode::Normal;
                }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) if c.is_ascii_digit() => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::AddWatch { name, command, buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    self.jobs.push(Job { name: name.clone(), command: command.clone(), schedule: Schedule { stype: "watch".into(), cron: None, interval_seconds: None, watch_path: Some(buf.clone()) }, enabled: true, description: String::new() });
                    save_jobs(&self.jobs); self.output = format!("✓ {} 추가 완료", name); self.mode = Mode::Normal;
                }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::Normal => {}
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => { if self.selected + 1 < self.jobs.len() { self.selected += 1; } }
            KeyCode::Char('a') => self.mode = Mode::AddName { buf: String::new() },
            KeyCode::Char('x') => {
                if let Some(job) = self.jobs.get(self.selected) {
                    let name = job.name.clone();
                    self.jobs.retain(|j| j.name != name);
                    save_jobs(&self.jobs);
                    self.output = format!("✓ {} 삭제", name);
                    if self.selected >= self.jobs.len() { self.selected = self.jobs.len().saturating_sub(1); }
                }
            }
            KeyCode::Char('t') => {
                if self.selected < self.jobs.len() {
                    self.jobs[self.selected].enabled = !self.jobs[self.selected].enabled;
                    save_jobs(&self.jobs);
                    let j = &self.jobs[self.selected];
                    self.output = format!("{} {}", j.name, if j.enabled { "활성화" } else { "비활성화" });
                }
            }
            KeyCode::Enter => {
                if let Some(job) = self.jobs.get(self.selected) {
                    self.output = format!("Running {}...\n", job.name);
                    let out = Command::new("bash").args(["-c", &job.command]).output();
                    match out {
                        Ok(o) => {
                            self.output.push_str(&String::from_utf8_lossy(&o.stdout));
                            self.output.push_str(&String::from_utf8_lossy(&o.stderr));
                        }
                        Err(e) => self.output.push_str(&format!("Error: {}", e)),
                    }
                }
            }
            KeyCode::Char('r') => { self.load().await?; self.output = "Refreshed.".into(); }
            _ => {}
        }
        Ok(())
    }
}
