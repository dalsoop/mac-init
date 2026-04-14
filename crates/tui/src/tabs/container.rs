use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::process::Command;

struct Container {
    name: String,
    status: String,
    image: String,
    ports: String,
}

pub struct ContainerTab {
    containers: Vec<Container>,
    orbstack_running: bool,
    orbstack_installed: bool,
    selected: usize,
    output: String,
}

fn cmd_ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).output().map(|o| o.status.success()).unwrap_or(false)
}

fn cmd_stdout(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

impl ContainerTab {
    pub fn new() -> Self {
        Self { containers: Vec::new(), orbstack_running: false, orbstack_installed: false, selected: 0, output: String::new() }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.orbstack_installed = cmd_ok("which", &["orbctl"]);
        self.orbstack_running = self.orbstack_installed && cmd_stdout("orbctl", &["status"]).contains("Running");
        self.containers.clear();

        if self.orbstack_running {
            let out = cmd_stdout("docker", &["ps", "-a", "--format", "{{.Names}}\t{{.Status}}\t{{.Image}}\t{{.Ports}}"]);
            for line in out.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    self.containers.push(Container {
                        name: parts[0].to_string(),
                        status: parts[1].to_string(),
                        image: parts[2].to_string(),
                        ports: parts.get(3).unwrap_or(&"").to_string(),
                    });
                }
            }
        }
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

        // Top: orbstack status
        let orb_status = if !self.orbstack_installed {
            "OrbStack 미설치 (i: 설치)"
        } else if self.orbstack_running {
            "OrbStack ✓ Running"
        } else {
            "OrbStack ✗ Stopped (u: 시작)"
        };
        let top_style = if self.orbstack_running { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };
        frame.render_widget(
            Paragraph::new(format!(" {} | {} containers", orb_status, self.containers.len()))
                .block(Block::default().borders(Borders::ALL).border_style(top_style).title(" Container ")),
            left[0],
        );

        // Table
        let vis = left[1].height.saturating_sub(2) as usize;
        let scroll = self.selected.saturating_sub(vis.saturating_sub(1));
        let rows: Vec<Row> = self.containers.iter().skip(scroll).take(vis).enumerate().map(|(vi, c)| {
            let sel = scroll + vi == self.selected;
            let base = if sel { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() };
            let running = c.status.starts_with("Up");
            let st = if running { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };
            Row::new(vec![
                Cell::from(if running { "✓" } else { "✗" }).style(if sel { base } else { st }),
                Cell::from(c.name.as_str()).style(base),
                Cell::from(c.status.as_str()).style(base),
                Cell::from(c.image.as_str()).style(base.fg(Color::DarkGray)),
            ])
        }).collect();

        let header = Row::new(vec!["", "Name", "Status", "Image"]).style(Style::default().fg(Color::Yellow).bold());
        frame.render_widget(
            Table::new(rows, [Constraint::Length(2), Constraint::Min(15), Constraint::Length(20), Constraint::Min(15)])
                .header(header).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Containers ")),
            left[1],
        );

        // Right
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Length(13), Constraint::Min(0)])
            .split(chunks[1]);

        let detail = self.containers.get(self.selected).map(|c| {
            format!("Name: {}\nStatus: {}\nImage: {}\nPorts: {}", c.name, c.status, c.image, if c.ports.is_empty() { "none" } else { &c.ports })
        }).unwrap_or_else(|| {
            if !self.orbstack_installed { "OrbStack 미설치\n  i: 설치".into() }
            else if !self.orbstack_running { "OrbStack 정지됨\n  u: 시작".into() }
            else { "컨테이너 없음".into() }
        });
        frame.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Details ")),
            right[0],
        );

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
                Line::from(""),
                Line::from(vec![Span::styled("  s", Style::default().fg(Color::Green).bold()), Span::raw("  Start container")]),
                Line::from(vec![Span::styled("  S", Style::default().fg(Color::Red).bold()), Span::raw("  Stop container")]),
                Line::from(vec![Span::styled("  R", Style::default().fg(Color::Cyan).bold()), Span::raw("  Restart container")]),
                Line::from(vec![Span::styled("  u", Style::default().fg(Color::Cyan).bold()), Span::raw("  OrbStack start")]),
                Line::from(vec![Span::styled("  d", Style::default().fg(Color::Cyan).bold()), Span::raw("  OrbStack stop")]),
                Line::from(vec![Span::styled("  i", Style::default().fg(Color::Cyan).bold()), Span::raw("  Install OrbStack")]),
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
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => { if self.selected + 1 < self.containers.len() { self.selected += 1; } }
            KeyCode::Char('s') => {
                if let Some(c) = self.containers.get(self.selected) {
                    let name = c.name.clone();
                    let ok = Command::new("docker").args(["start", &name]).output().map(|o| o.status.success()).unwrap_or(false);
                    self.output = format!("{} {}", name, if ok { "✓ 시작" } else { "✗ 실패" });
                    self.load().await?;
                }
            }
            KeyCode::Char('S') => {
                if let Some(c) = self.containers.get(self.selected) {
                    let name = c.name.clone();
                    let ok = Command::new("docker").args(["stop", &name]).output().map(|o| o.status.success()).unwrap_or(false);
                    self.output = format!("{} {}", name, if ok { "✓ 정지" } else { "✗ 실패" });
                    self.load().await?;
                }
            }
            KeyCode::Char('R') => {
                if let Some(c) = self.containers.get(self.selected) {
                    let name = c.name.clone();
                    let ok = Command::new("docker").args(["restart", &name]).output().map(|o| o.status.success()).unwrap_or(false);
                    self.output = format!("{} {}", name, if ok { "✓ 재시작" } else { "✗ 실패" });
                    self.load().await?;
                }
            }
            KeyCode::Char('u') => {
                self.output = "OrbStack 시작 중...".into();
                let ok = Command::new("orbctl").args(["start"]).output().map(|o| o.status.success()).unwrap_or(false);
                if !ok { let _ = Command::new("open").args(["-a", "OrbStack"]).output(); }
                self.output = "✓ OrbStack 시작".into();
                self.load().await?;
            }
            KeyCode::Char('d') => {
                let ok = Command::new("orbctl").args(["stop"]).output().map(|o| o.status.success()).unwrap_or(false);
                self.output = if ok { "✓ OrbStack 정지" } else { "✗ 실패" }.into();
                self.load().await?;
            }
            KeyCode::Char('i') => {
                if self.orbstack_installed {
                    self.output = "✓ OrbStack 이미 설치됨".into();
                } else {
                    self.output = "OrbStack 설치 중...\n".into();
                    let ok = Command::new("brew").args(["install", "--cask", "orbstack"]).output().map(|o| o.status.success()).unwrap_or(false);
                    self.output.push_str(if ok { "✓ 설치 완료" } else { "✗ 설치 실패" });
                    self.load().await?;
                }
            }
            KeyCode::Char('r') => { self.load().await?; self.output = "Refreshed.".into(); }
            _ => {}
        }
        Ok(())
    }
}
