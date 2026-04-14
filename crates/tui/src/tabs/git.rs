use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct GitStatus {
    git_version: String,
    user_name: String,
    user_email: String,
    ssh_keys: Vec<SshKey>,
    ssh_hosts: Vec<String>,
    gh_installed: bool,
    gh_version: String,
    gh_authed: bool,
    gh_user: String,
    git_lfs: bool,
}

struct SshKey {
    name: String,
    fingerprint: String,
}

enum Mode {
    Normal,
    EditName { buf: String },
    EditEmail { buf: String },
}

pub struct GitTab {
    status: Option<GitStatus>,
    selected: usize,
    output: String,
    mode: Mode,
}

fn home() -> String { std::env::var("HOME").unwrap_or_default() }
fn cmd_out(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}
fn cmd_ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).output().map(|o| o.status.success()).unwrap_or(false)
}

fn load_status() -> GitStatus {
    let git_version = cmd_out("git", &["--version"]);
    let user_name = cmd_out("git", &["config", "--global", "user.name"]);
    let user_email = cmd_out("git", &["config", "--global", "user.email"]);

    let ssh_dir = PathBuf::from(home()).join(".ssh");
    let mut ssh_keys = Vec::new();
    for kt in &["id_ed25519", "id_rsa", "id_ecdsa"] {
        if ssh_dir.join(kt).exists() {
            let fp = cmd_out("ssh-keygen", &["-lf", &ssh_dir.join(format!("{}.pub", kt)).to_string_lossy()]);
            let short = fp.split_whitespace().nth(1).unwrap_or("?").to_string();
            ssh_keys.push(SshKey { name: kt.to_string(), fingerprint: short });
        }
    }

    let ssh_hosts = if ssh_dir.join("config").exists() {
        fs::read_to_string(ssh_dir.join("config")).unwrap_or_default()
            .lines().filter(|l| l.trim().starts_with("Host "))
            .map(|l| l.trim().strip_prefix("Host ").unwrap_or("").to_string())
            .collect()
    } else { Vec::new() };

    let gh_version = cmd_out("gh", &["--version"]);
    let gh_installed = !gh_version.is_empty();
    let gh_authed = gh_installed && cmd_ok("gh", &["auth", "token"]);
    let gh_user = if gh_authed { cmd_out("gh", &["api", "user", "-q", ".login"]) } else { String::new() };
    let git_lfs = cmd_ok("git", &["lfs", "version"]);

    GitStatus { git_version, user_name, user_email, ssh_keys, ssh_hosts, gh_installed, gh_version, gh_authed, gh_user, git_lfs }
}

const ITEMS: &[&str] = &["Profile: name", "Profile: email", "SSH Keys", "SSH Config", "GitHub CLI", "GitHub Auth", "GitHub SSH", "Git LFS"];

impl GitTab {
    pub fn new() -> Self {
        Self { status: None, selected: 0, output: String::new(), mode: Mode::Normal }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.status = Some(load_status());
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

        let top = match &self.mode {
            Mode::Normal => " Git Settings".into(),
            Mode::EditName { buf } => format!("[user.name] {}", buf),
            Mode::EditEmail { buf } => format!("[user.email] {}", buf),
        };
        let top_style = match &self.mode {
            Mode::Normal => Style::default().fg(Color::DarkGray),
            _ => Style::default().fg(Color::Green),
        };
        frame.render_widget(
            Paragraph::new(top).block(Block::default().borders(Borders::ALL).border_style(top_style).title(" Git ")),
            left[0],
        );

        let s = self.status.as_ref();
        let items: Vec<ListItem> = ITEMS.iter().enumerate().map(|(i, label)| {
            let sel = i == self.selected;
            let base = if sel { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() };

            let (icon, value) = match (i, s) {
                (0, Some(s)) => (if s.user_name.is_empty() { "✗" } else { "✓" }, s.user_name.clone()),
                (1, Some(s)) => (if s.user_email.is_empty() { "✗" } else { "✓" }, s.user_email.clone()),
                (2, Some(s)) => (if s.ssh_keys.is_empty() { "✗" } else { "✓" }, format!("{}개", s.ssh_keys.len())),
                (3, Some(s)) => (if s.ssh_hosts.is_empty() { "✗" } else { "✓" }, format!("{}개 호스트", s.ssh_hosts.len())),
                (4, Some(s)) => (if s.gh_installed { "✓" } else { "✗" }, if s.gh_installed { s.gh_version.lines().next().unwrap_or("").to_string() } else { "미설치".into() }),
                (5, Some(s)) => (if s.gh_authed { "✓" } else { "✗" }, if s.gh_authed { s.gh_user.clone() } else { "미인증".into() }),
                (6, Some(s)) => (if !s.ssh_keys.is_empty() && s.gh_authed { "○" } else { "✗" }, "SSH 키 → GitHub".into()),
                (7, Some(s)) => (if s.git_lfs { "✓" } else { "✗" }, if s.git_lfs { "설치됨".into() } else { "미설치".into() }),
                _ => ("?", "로딩 중...".into()),
            };

            let icon_style = if icon == "✓" { Style::default().fg(Color::Green) }
                else if icon == "✗" { Style::default().fg(Color::Red) }
                else { Style::default().fg(Color::Yellow) };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), if sel { base } else { icon_style }),
                Span::styled(format!("{:<20}", label), base),
                Span::styled(value, base.fg(Color::Gray)),
            ]))
        }).collect();

        frame.render_widget(
            List::new(items).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Status ")),
            left[1],
        );

        // Right
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(chunks[1]);

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
                Line::from(""),
                Line::from(vec![Span::styled("  Enter", Style::default().fg(Color::Green).bold()), Span::raw("  Edit/Setup selected")]),
                Line::from(vec![Span::styled("  r", Style::default().fg(Color::Cyan).bold()), Span::raw("  Refresh")]),
                Line::from(""),
                Line::from(Span::styled(" Selected에 따라:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("  name/email → 값 수정", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("  SSH Keys → 키 생성", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("  GitHub CLI → 설치", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("  GitHub Auth → 인증", Style::default().fg(Color::DarkGray))),
            ]).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Actions ")),
            right[0],
        );

        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Output ")),
            right[1],
        );
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            Mode::EditName { buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    Command::new("git").args(["config", "--global", "user.name", buf]).output().ok();
                    self.output = format!("✓ user.name = {}", buf);
                    self.mode = Mode::Normal;
                    self.load().await?;
                }
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            } return Ok(()); }
            Mode::EditEmail { buf } => { match key.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    Command::new("git").args(["config", "--global", "user.email", buf]).output().ok();
                    self.output = format!("✓ user.email = {}", buf);
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
            KeyCode::Down | KeyCode::Char('j') => { if self.selected + 1 < ITEMS.len() { self.selected += 1; } }
            KeyCode::Enter => {
                match self.selected {
                    0 => { // Edit name
                        let current = self.status.as_ref().map(|s| s.user_name.clone()).unwrap_or_default();
                        self.mode = Mode::EditName { buf: current };
                    }
                    1 => { // Edit email
                        let current = self.status.as_ref().map(|s| s.user_email.clone()).unwrap_or_default();
                        self.mode = Mode::EditEmail { buf: current };
                    }
                    2 => { // SSH key setup
                        self.output = "SSH 키 생성은 CLI에서: mac run git ssh-setup".into();
                    }
                    4 => { // gh install
                        if !self.status.as_ref().map(|s| s.gh_installed).unwrap_or(false) {
                            self.output = "설치 중...\n".into();
                            let out = Command::new("brew").args(["install", "gh"]).output();
                            self.output.push_str(match out {
                                Ok(o) if o.status.success() => "✓ gh 설치 완료",
                                _ => "✗ 설치 실패",
                            });
                            self.load().await?;
                        } else {
                            self.output = "✓ gh 이미 설치됨".into();
                        }
                    }
                    5 => { // gh auth
                        self.output = "GitHub 인증은 CLI에서: mac run git gh-auth".into();
                    }
                    6 => { // gh ssh setup
                        self.output = "SSH 키 등록은 CLI에서: mac run git gh-ssh-setup".into();
                    }
                    7 => { // git lfs
                        if !self.status.as_ref().map(|s| s.git_lfs).unwrap_or(false) {
                            self.output = "설치 중...\n".into();
                            let out = Command::new("brew").args(["install", "git-lfs"]).output();
                            self.output.push_str(match out {
                                Ok(o) if o.status.success() => "✓ git-lfs 설치 완료",
                                _ => "✗ 설치 실패",
                            });
                            let _ = Command::new("git").args(["lfs", "install"]).output();
                            self.load().await?;
                        } else {
                            self.output = "✓ git-lfs 이미 설치됨".into();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('r') => { self.load().await?; self.output = "Refreshed.".into(); }
            _ => {}
        }
        Ok(())
    }
}
