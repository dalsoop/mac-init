use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind, MouseButton};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tui_tree_widget::{Tree, TreeItem, TreeState};

#[derive(Clone)]
pub enum NodeKind {
    Category,                                  // 그룹 헤더 (Domains, Apps, Extensions)
    Domain { name: String, installed: bool },
    App { name: String, cask: String, installed: bool },
    Extension { id: String, installed: bool },
}

#[derive(Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub children: Vec<Node>,
}

const KNOWN_DOMAINS: &[(&str, &str)] = &[
    ("bootstrap", "최초 의존성 설치 (brew, gh, dotenvx, rust, nickel)"),
    ("keyboard", "Caps Lock → F18 한영 전환"),
    ("connect", "외부 서비스 연결 관리 (.env + dotenvx)"),
    ("container", "Docker/OrbStack 컨테이너 관리"),
    ("wireguard", "WireGuard VPN 관리"),
    ("quickaction", "Finder 우클릭 Quick Actions"),
    ("vscode", "VS Code 설치, 확장, 설정 관리"),
    ("git", "Git 프로필, SSH 키, GitHub CLI"),
    ("cron", "LaunchAgents 스케줄 관리"),
    ("defaults", "macOS 시스템 설정"),
    ("dotfiles", "설정 파일 스캔/읽기"),
    ("files", "파일 자동 분류, SD 백업"),
    ("projects", "프로젝트 스캔/동기화"),
    ("worktree", "Git worktree 관리"),
];

// (display name, brew cask, app bundle name)
const KNOWN_APPS: &[(&str, &str, &str)] = &[
    ("Visual Studio Code", "visual-studio-code", "Visual Studio Code"),
    ("OrbStack", "orbstack", "OrbStack"),
    ("WireGuard", "wireguard", "WireGuard"),
    ("iTerm2", "iterm2", "iTerm"),
    ("Obsidian", "obsidian", "Obsidian"),
    ("Karabiner-Elements", "karabiner-elements", "Karabiner-Elements"),
];

fn app_installed(bundle_name: &str) -> bool {
    // 1. Direct path check
    if std::path::Path::new(&format!("/Applications/{}.app", bundle_name)).exists() {
        return true;
    }
    if let Ok(home) = std::env::var("HOME") {
        if std::path::Path::new(&format!("{}/Applications/{}.app", home, bundle_name)).exists() {
            return true;
        }
    }
    false
}

fn brew_installed_casks() -> std::collections::HashSet<String> {
    let output = Command::new("brew").args(["list", "--cask"]).output();
    output.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).collect())
        .unwrap_or_default()
}

pub struct InstallTab {
    state: TreeState<String>,
    nodes: Vec<Node>,
    output: String,
    last_area: Rect,
}

fn home() -> String { std::env::var("HOME").unwrap_or_default() }

fn mac_bin_path() -> String {
    // Try common install locations in priority order
    let candidates = [
        format!("{}/.cargo/bin/mac", home()),
        format!("{}/.local/bin/mac", home()),
        "/usr/local/bin/mac".to_string(),
        "/opt/homebrew/bin/mac".to_string(),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.clone();
        }
    }
    "mac".to_string()
}

fn registry_installed() -> Vec<String> {
    let path = PathBuf::from(home()).join(".mac-app-init/domains/registry.json");
    if !path.exists() { return Vec::new(); }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    json.get("installed").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|d| d.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
        .unwrap_or_default()
}

fn vscode_extensions() -> Vec<String> {
    let output = Command::new("code").args(["--list-extensions"]).output();
    output.ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().map(|l| l.to_string()).collect()).unwrap_or_default()
}

impl InstallTab {
    pub fn new() -> Self {
        let mut state = TreeState::default();
        state.open(vec!["domains".into()]);
        Self { state, nodes: Vec::new(), output: String::new(), last_area: Rect::default() }
    }

    pub async fn load(&mut self) -> Result<()> {
        let installed_domains = registry_installed();
        let installed_set: HashSet<&str> = installed_domains.iter().map(|s| s.as_str()).collect();

        // Domains
        let domain_children: Vec<Node> = KNOWN_DOMAINS.iter().map(|(name, desc)| {
            let installed = installed_set.contains(name);
            Node {
                id: format!("domain:{}", name),
                label: format!("{} {} — {}", if installed { "✓" } else { " " }, name, desc),
                kind: NodeKind::Domain { name: name.to_string(), installed },
                children: vec![],
            }
        }).collect();

        let installed_count = domain_children.iter().filter(|n| matches!(&n.kind, NodeKind::Domain { installed: true, .. })).count();
        let domains = Node {
            id: "domains".into(),
            label: format!("Domains ({}/{})", installed_count, KNOWN_DOMAINS.len()),
            kind: NodeKind::Category,
            children: domain_children,
        };

        // Apps — check via brew cask + .app file
        let brew_casks = brew_installed_casks();
        let app_children: Vec<Node> = KNOWN_APPS.iter().map(|(name, cask, bundle)| {
            let installed = brew_casks.contains(*cask) || app_installed(bundle);
            Node {
                id: format!("app:{}", cask),
                label: format!("{} {}", if installed { "✓" } else { " " }, name),
                kind: NodeKind::App { name: name.to_string(), cask: cask.to_string(), installed },
                children: vec![],
            }
        }).collect();

        let app_installed_cnt = app_children.iter().filter(|n| matches!(&n.kind, NodeKind::App { installed: true, .. })).count();
        let apps = Node {
            id: "apps".into(),
            label: format!("Apps ({}/{})", app_installed_cnt, KNOWN_APPS.len()),
            kind: NodeKind::Category,
            children: app_children,
        };

        // Extensions
        let exts = vscode_extensions();
        let ext_children: Vec<Node> = exts.iter().map(|id| {
            Node {
                id: format!("ext:{}", id),
                label: format!("✓ {}", id),
                kind: NodeKind::Extension { id: id.clone(), installed: true },
                children: vec![],
            }
        }).collect();
        let extensions = Node {
            id: "extensions".into(),
            label: format!("VS Code Extensions ({})", exts.len()),
            kind: NodeKind::Category,
            children: ext_children,
        };

        self.nodes = vec![domains, apps, extensions];
        Ok(())
    }

    fn build_tree_items(&self) -> Vec<TreeItem<'static, String>> {
        self.nodes.iter().map(|n| node_to_item(n)).collect()
    }

    fn find_node(&self, id: &str) -> Option<&Node> {
        for top in &self.nodes {
            if top.id == id { return Some(top); }
            for child in &top.children {
                if child.id == id { return Some(child); }
            }
        }
        None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.last_area = area;
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let items = self.build_tree_items();
        let tree = Tree::new(&items)
            .expect("tree items")
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))
                .title(" Install (Space=expand, Enter=action, mouse 클릭) "))
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White).bold());

        frame.render_stateful_widget(tree, chunks[0], &mut self.state);

        // Right panel
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(chunks[1]);

        let selected_id = self.state.selected().last().cloned().unwrap_or_default();
        let detail = if let Some(node) = self.find_node(&selected_id) {
            match &node.kind {
                NodeKind::Category => format!(" {} (카테고리)\n\n  Space로 펼치기/접기", node.label),
                NodeKind::Domain { name, installed } => {
                    format!(" Domain: {}\n\n  상태: {}\n\n  Enter:\n    {} 설치/삭제 토글", name,
                        if *installed { "✓ 설치됨" } else { "✗ 미설치" },
                        if *installed { "x" } else { "i" })
                }
                NodeKind::App { name, cask, installed } => {
                    format!(" App: {}\n\n  Cask: {}\n  상태: {}\n\n  Enter: {}", name, cask,
                        if *installed { "✓ 설치됨" } else { "✗ 미설치" },
                        if *installed { "삭제 (brew uninstall)" } else { "설치 (brew install)" })
                }
                NodeKind::Extension { id, .. } => {
                    format!(" Extension: {}\n\n  Enter: 제거 (code --uninstall-extension)", id)
                }
            }
        } else {
            "선택된 항목 없음".into()
        };

        frame.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Details ")),
            right[0],
        );

        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Output ")),
            right[1],
        );
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => { self.state.key_up(); }
            KeyCode::Down | KeyCode::Char('j') => { self.state.key_down(); }
            KeyCode::Left | KeyCode::Char('h') => { self.state.key_left(); }
            KeyCode::Right | KeyCode::Char('l') => { self.state.key_right(); }
            KeyCode::Char(' ') => { self.state.toggle_selected(); }
            KeyCode::Enter => self.activate_selected().await?,
            KeyCode::Char('r') => { self.load().await?; self.output = "Refreshed.".into(); }
            _ => {}
        }
        Ok(())
    }

    pub async fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let _ = self.state.click_at(Position::new(mouse.column, mouse.row));
            }
            MouseEventKind::ScrollUp => { self.state.scroll_up(3); }
            MouseEventKind::ScrollDown => { self.state.scroll_down(3); }
            _ => {}
        }
        Ok(())
    }

    async fn activate_selected(&mut self) -> Result<()> {
        let selected_id = match self.state.selected().last() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        let node = match self.find_node(&selected_id) {
            Some(n) => n.clone(),
            None => return Ok(()),
        };
        match node.kind {
            NodeKind::Category => { self.state.toggle(self.state.selected().to_vec()); }
            NodeKind::Domain { name, installed } => {
                let action = if installed { "remove" } else { "install" };
                self.output = format!("{} {}...", action, name);
                let mac_bin = mac_bin_path();
                let out = Command::new(&mac_bin).args([action, &name]).output();
                self.output = match out {
                    Ok(o) if o.status.success() => format!(
                        "✓ {} {} 완료\n{}",
                        name,
                        if action == "install" { "설치" } else { "삭제" },
                        String::from_utf8_lossy(&o.stdout).trim()
                    ),
                    Ok(o) => format!(
                        "✗ {} 실패\nstdout: {}\nstderr: {}",
                        name,
                        String::from_utf8_lossy(&o.stdout).trim(),
                        String::from_utf8_lossy(&o.stderr).trim()
                    ),
                    Err(e) => format!("✗ '{}' 실행 실패: {}", mac_bin, e),
                };
                self.load().await?;
            }
            NodeKind::App { name, cask, installed } => {
                self.output = format!("{}ing {}...", if installed { "Uninstall" } else { "Install" }, name);
                let args: Vec<&str> = if installed {
                    vec!["uninstall", "--cask", &cask]
                } else {
                    vec!["install", "--cask", &cask]
                };
                let out = Command::new("brew").args(&args).output();
                self.output = match out {
                    Ok(o) => format!("{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)),
                    Err(e) => format!("Error: {}", e),
                };
                self.load().await?;
            }
            NodeKind::Extension { id, .. } => {
                self.output = format!("Removing {}...", id);
                let out = Command::new("code").args(["--uninstall-extension", &id]).output();
                self.output = match out {
                    Ok(o) => format!("{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)),
                    Err(e) => format!("Error: {}", e),
                };
                self.load().await?;
            }
        }
        Ok(())
    }
}

fn node_to_item(node: &Node) -> TreeItem<'static, String> {
    if node.children.is_empty() {
        TreeItem::new_leaf(node.id.clone(), node.label.clone())
    } else {
        let children: Vec<TreeItem<String>> = node.children.iter().map(node_to_item).collect();
        TreeItem::new(node.id.clone(), node.label.clone(), children).expect("tree")
    }
}
