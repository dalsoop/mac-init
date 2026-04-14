use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct DomainInfo {
    name: String,
    description: String,
    installed: bool,
    version: String,
}

pub struct StoreTab {
    domains: Vec<DomainInfo>,
    selected: usize,
    output: String,
}

fn domains_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".mac-app-init/domains")
}

fn registry_path() -> PathBuf {
    domains_dir().join("registry.json")
}

fn load_installed() -> Vec<(String, String)> {
    let path = registry_path();
    if !path.exists() {
        return Vec::new();
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    json.get("installed")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    let name = d.get("name")?.as_str()?.to_string();
                    let version = d.get("version")?.as_str()?.to_string();
                    Some((name, version))
                })
                .collect()
        })
        .unwrap_or_default()
}

const KNOWN: &[(&str, &str)] = &[
    ("keyboard", "Caps Lock → F18 한영 전환"),
    ("brew", "Homebrew 패키지 관리"),
    ("connect", "외부 서비스 연결 관리 (.env + dotenvx)"),
    ("cron", "LaunchAgents 스케줄 관리"),
    ("defaults", "macOS 시스템 설정"),
    ("dotfiles", "설정 파일 스캔/읽기"),
    ("files", "파일 자동 분류, SD 백업"),
    ("projects", "프로젝트 스캔/동기화"),
    ("worktree", "Git worktree 관리"),
    ("mount", "sshfs/SMB 마운트"),
    ("network", "VPN, 연결 체크"),
    ("ssh", "SSH 키/연결 관리"),
    ("proxmox", "Proxmox VM/LXC"),
    ("synology", "Synology NAS 관리"),
    ("setup", "macFUSE, sshfs 설치"),
    ("workspace", "tmux, CLI 도구, 셸"),
    ("github", "gh CLI, SSH 키"),
    ("obsidian", "Obsidian 볼트 관리"),
    ("veil", "VeilKey 시크릿 관리"),
    ("openclaw", "OpenClaw AI 어시스턴트"),
    ("dal", "Dalcenter AI 에이전트"),
];

impl StoreTab {
    pub fn new() -> Self {
        Self {
            domains: Vec::new(),
            selected: 0,
            output: String::new(),
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        let installed = load_installed();
        self.domains = KNOWN
            .iter()
            .map(|(name, desc)| {
                let inst = installed.iter().find(|(n, _)| n == name);
                DomainInfo {
                    name: name.to_string(),
                    description: desc.to_string(),
                    installed: inst.is_some(),
                    version: inst.map(|(_, v)| v.clone()).unwrap_or_default(),
                }
            })
            .collect();
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        // Left: domain list
        let visible_height = chunks[0].height.saturating_sub(2) as usize;
        let scroll = self.selected.saturating_sub(visible_height.saturating_sub(1));

        let rows: Vec<Row> = self
            .domains
            .iter()
            .skip(scroll)
            .take(visible_height)
            .enumerate()
            .map(|(vis_i, d)| {
                let is_selected = scroll + vis_i == self.selected;
                let base = if is_selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };
                let status = if d.installed { "✓" } else { " " };
                let status_style = if d.installed {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                Row::new(vec![
                    Cell::from(status).style(if is_selected { base } else { status_style }),
                    Cell::from(d.name.as_str()).style(base),
                    Cell::from(d.description.as_str()).style(base.fg(Color::Gray)),
                ])
            })
            .collect();

        let installed_count = self.domains.iter().filter(|d| d.installed).count();
        let header = Row::new(vec!["", "Domain", "Description"])
            .style(Style::default().fg(Color::Yellow).bold());

        let table = Table::new(rows, [
            Constraint::Length(2),
            Constraint::Length(15),
            Constraint::Min(20),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" Domain Store ({}/{}) ", installed_count, self.domains.len())),
        );
        frame.render_widget(table, chunks[0]);

        // Right: detail + actions + output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Min(0),
            ])
            .split(chunks[1]);

        // Detail
        let detail = if let Some(d) = self.domains.get(self.selected) {
            format!(
                "Name: {}\nDescription: {}\nStatus: {}\n{}",
                d.name,
                d.description,
                if d.installed {
                    format!("Installed ({})", d.version)
                } else {
                    "Not installed".to_string()
                },
                if d.installed {
                    format!("\nBinary: ~/.mac-app-init/domains/mac-domain-{}", d.name)
                } else {
                    String::new()
                }
            )
        } else {
            String::new()
        };
        frame.render_widget(
            Paragraph::new(detail)
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Details "),
                ),
            right[0],
        );

        // Actions
        let selected_installed = self.domains.get(self.selected).map(|d| d.installed).unwrap_or(false);
        let actions = Paragraph::new(vec![
            Line::from(Span::styled(" Actions", Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            if selected_installed {
                Line::from(vec![
                    Span::styled("  d", Style::default().fg(Color::Red).bold()),
                    Span::raw("  Remove"),
                ])
            } else {
                Line::from(vec![
                    Span::styled("  i", Style::default().fg(Color::Green).bold()),
                    Span::raw("  Install"),
                ])
            },
            Line::from(vec![
                Span::styled("  u", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Update"),
            ]),
            Line::from(vec![
                Span::styled("  U", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Update all"),
            ]),
            Line::from(vec![
                Span::styled("  r", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Refresh"),
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
        frame.render_widget(
            Paragraph::new(self.output.as_str())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Output "),
                ),
            right[2],
        );
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.domains.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Char('i') | KeyCode::Enter => {
                if let Some(d) = self.domains.get(self.selected) {
                    if !d.installed {
                        let name = d.name.clone();
                        self.output = format!("Installing {}...\n", name);
                        let out = run_mac(&["install", &name]);
                        self.output.push_str(&out);
                        self.load().await?;
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(d) = self.domains.get(self.selected) {
                    if d.installed {
                        let name = d.name.clone();
                        self.output = format!("Removing {}...\n", name);
                        let out = run_mac(&["remove", &name]);
                        self.output.push_str(&out);
                        self.load().await?;
                    }
                }
            }
            KeyCode::Char('u') => {
                if let Some(d) = self.domains.get(self.selected) {
                    if d.installed {
                        let name = d.name.clone();
                        self.output = format!("Updating {}...\n", name);
                        let out = run_mac(&["update", &name]);
                        self.output.push_str(&out);
                        self.load().await?;
                    }
                }
            }
            KeyCode::Char('U') => {
                self.output = "Updating all...\n".to_string();
                let out = run_mac(&["update-all"]);
                self.output.push_str(&out);
                self.load().await?;
            }
            KeyCode::Char('r') => {
                self.load().await?;
                self.output = "Refreshed.".to_string();
            }
            _ => {}
        }
        Ok(())
    }
}

fn run_mac(args: &[&str]) -> String {
    // Try local binary first, then PATH
    let mac_bin = domains_dir().parent().map(|p| p.join("mac")).unwrap_or_default();
    let cmd = if mac_bin.exists() {
        mac_bin.to_string_lossy().to_string()
    } else {
        "mac".to_string()
    };
    match Command::new(&cmd).args(args).output() {
        Ok(o) => format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => format!("Error: {}\nmac 바이너리를 찾을 수 없습니다.\n  cargo install --path crates/domains/manager", e),
    }
}
