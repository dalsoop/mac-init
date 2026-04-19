use crate::registry::{available_domains, fetch_spec, install_domain, installed_domains, remove_domain, run_action};
use crate::spec::{DomainSpec, Section};
use crate::widgets;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{prelude::*, widgets::*};

/// 사이드바 그룹 정의. 순서 = 화면 표시 순서.
const GROUPS: &[(&str, &str)] = &[
    ("init",   "인입"),
    ("infra",  "인프라"),
    ("auto",   "자동화"),
    ("dev",    "개발"),
    ("finder", "Finder"),
    ("system", "시스템"),
    ("other",  "기타"),
];

/// 도메인 → 그룹 매핑. spec.group 이 없으면 여기서 fallback.
fn default_group(domain: &str) -> &'static str {
    match domain {
        "mount" | "env" | "host" | "network" | "ssh" | "proxmox" | "synology" => "infra",
        "cron" | "files" | "sd-backup" => "auto",
        "git" | "vscode" | "container" => "dev",
        "quickaction" => "finder",
        "keyboard" | "shell" | "bootstrap" | "wireguard" => "system",
        _ => "other",
    }
}

/// 포커스가 사이드바인지 콘텐츠인지.
#[derive(PartialEq, Clone, Copy)]
pub enum Focus {
    Sidebar,
    Content,
}

pub struct App {
    pub should_quit: bool,
    pub domains: Vec<String>,
    pub specs: Vec<Option<DomainSpec>>,
    pub available: Vec<String>,
    pub install_focus: usize,
    pub install_area_top: u16,
    pub selected_tab: usize,            // 0 = Install, 1+ = 도메인 (flat index)
    pub focus_button: usize,
    pub output: String,
    pub sidebar_entries: Vec<SidebarEntry>,
    /// 사이드바 커서 위치 (sidebar_entries 인덱스)
    pub sidebar_cursor: usize,
    /// 접힌 그룹 이름
    pub collapsed_groups: std::collections::HashSet<String>,
    /// 현재 포커스 뎁스
    pub focus: Focus,
}

#[derive(Clone)]
pub enum SidebarEntry {
    /// 설치 관리
    Install,
    /// 그룹 헤더 — 접기/펼치기 가능
    GroupHeader { label: String, group_id: String },
    /// 도메인
    Domain { idx: usize, label: String },
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            domains: Vec::new(),
            specs: Vec::new(),
            available: Vec::new(),
            install_focus: 0,
            install_area_top: 0,
            selected_tab: 0,
            focus_button: 0,
            output: String::new(),
            sidebar_entries: Vec::new(),
            sidebar_cursor: 0,
            collapsed_groups: std::collections::HashSet::new(),
            focus: Focus::Sidebar,
        }
    }

    pub fn load(&mut self) {
        self.domains = installed_domains();
        self.specs = self.domains.iter().map(|d| fetch_spec(d)).collect();
        self.available = available_domains();
        self.build_sidebar();
        if self.selected_tab > self.domains.len() {
            self.selected_tab = 0;
        }
        if self.install_focus >= self.available.len() {
            self.install_focus = self.available.len().saturating_sub(1);
        }
    }

    /// 현재 탭의 refresh_interval (초). 0 이면 자동 갱신 없음.
    pub fn current_refresh_interval(&self) -> u32 {
        if self.selected_tab == 0 { return 0; }
        let idx = self.selected_tab - 1;
        self.specs.get(idx)
            .and_then(|s| s.as_ref())
            .map(|s| s.refresh_interval)
            .unwrap_or(0)
    }

    /// 현재 탭의 spec 만 재로드.
    pub fn refresh_current_tab(&mut self) {
        if self.selected_tab == 0 { return; }
        let idx = self.selected_tab - 1;
        if let Some(domain) = self.domains.get(idx) {
            self.specs[idx] = crate::registry::fetch_spec(domain);
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sidebar_cursor > 0 { self.sidebar_cursor -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.sidebar_cursor + 1 < self.sidebar_entries.len() {
                    self.sidebar_cursor += 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                // 뎁스 이동: 사이드바 → 콘텐츠
                self.focus = Focus::Content;
                self.focus_button = 0;
            }
            KeyCode::Enter => {
                // 그룹 헤더 → 접기/펼치기, 도메인/Install → 선택 + 콘텐츠로
                if let Some(entry) = self.sidebar_entries.get(self.sidebar_cursor).cloned() {
                    match entry {
                        SidebarEntry::GroupHeader { group_id, .. } => {
                            if self.collapsed_groups.contains(&group_id) {
                                self.collapsed_groups.remove(&group_id);
                            } else {
                                self.collapsed_groups.insert(group_id);
                            }
                            self.build_sidebar();
                        }
                        SidebarEntry::Install => {
                            self.selected_tab = 0;
                            self.focus = Focus::Content;
                            self.focus_button = 0;
                        }
                        SidebarEntry::Domain { idx, .. } => {
                            self.selected_tab = idx + 1;
                            self.focus = Focus::Content;
                            self.focus_button = 0;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_content_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => {
                // 뎁스 이동: 콘텐츠 → 사이드바
                self.focus = Focus::Sidebar;
                return;
            }
            _ => {}
        }

        if self.selected_tab == 0 {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.install_focus = self.install_focus.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.install_focus + 1 < self.available.len() {
                        self.install_focus += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => self.toggle_install(),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.focus_button = self.focus_button.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.focus_button = self.focus_button.saturating_add(1);
                }
                KeyCode::Enter => self.activate_button(),
                KeyCode::Char(c) => self.activate_by_key(c),
                _ => {}
            }
        }
    }

    fn build_sidebar(&mut self) {
        let mut entries = Vec::new();

        for &(group_id, group_label) in GROUPS {
            let is_init = group_id == "init";

            let mut group_domains: Vec<(usize, String)> = Vec::new();
            for (i, domain) in self.domains.iter().enumerate() {
                let spec_group = self.specs[i].as_ref()
                    .and_then(|s| s.group.as_deref())
                    .unwrap_or_else(|| default_group(domain));
                if spec_group != group_id { continue; }
                let label = self.specs[i].as_ref()
                    .map(|s| s.tab.label_ko.as_deref().unwrap_or(&s.tab.label).to_string())
                    .unwrap_or_else(|| domain.clone());
                group_domains.push((i, label));
            }

            if !is_init && group_domains.is_empty() { continue; }

            let collapsed = self.collapsed_groups.contains(group_id);
            entries.push(SidebarEntry::GroupHeader {
                label: group_label.to_string(),
                group_id: group_id.to_string(),
            });

            if !collapsed {
                if is_init {
                    entries.push(SidebarEntry::Install);
                }
                for (idx, label) in group_domains {
                    entries.push(SidebarEntry::Domain { idx, label });
                }
            }
        }
        self.sidebar_entries = entries;
        if self.sidebar_cursor >= self.sidebar_entries.len() {
            self.sidebar_cursor = self.sidebar_entries.len().saturating_sub(1);
        }
    }

    fn is_installed(&self, name: &str) -> bool {
        self.domains.iter().any(|d| d == name)
    }

    fn toggle_install(&mut self) {
        let Some(name) = self.available.get(self.install_focus).cloned() else { return; };
        let msg = if self.is_installed(&name) {
            format!("Removing {}...\n", name)
        } else {
            format!("Installing {}...\n", name)
        };
        self.output = msg;
        let result = if self.is_installed(&name) {
            remove_domain(&name)
        } else {
            install_domain(&name)
        };
        self.output.push_str(&result);
        self.load();
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(0)])
            .split(frame.area());

        self.render_sidebar(frame, chunks[0]);
        self.render_content(frame, chunks[1]);
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let mut items = Vec::new();
        let is_focused = self.focus == Focus::Sidebar;

        for (i, entry) in self.sidebar_entries.iter().enumerate() {
            let cursor = is_focused && i == self.sidebar_cursor;
            match entry {
                SidebarEntry::Install => {
                    let selected = self.selected_tab == 0;
                    let style = if cursor {
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold()
                    } else if selected {
                        Style::default().bg(Color::Cyan).fg(Color::Black).bold()
                    } else {
                        Style::default().fg(Color::White)
                    };
                    items.push(ListItem::new(Line::from(Span::styled("  📥 설치 관리", style))));
                }
                SidebarEntry::GroupHeader { label, group_id } => {
                    let collapsed = self.collapsed_groups.contains(group_id);
                    let arrow = if collapsed { "▶" } else { "▼" };
                    let style = if cursor {
                        Style::default().bg(Color::DarkGray).fg(Color::Yellow).bold()
                    } else {
                        Style::default().fg(Color::DarkGray).bold()
                    };
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("{} {}", arrow, label), style,
                    ))));
                }
                SidebarEntry::Domain { idx, label } => {
                    let tab_idx = idx + 1;
                    let selected = self.selected_tab == tab_idx;
                    let icon = self.specs[*idx].as_ref()
                        .and_then(|s| s.tab.icon.as_deref()).unwrap_or("");
                    let style = if cursor {
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold()
                    } else if selected {
                        Style::default().bg(Color::Cyan).fg(Color::Black).bold()
                    } else {
                        Style::default().fg(Color::White)
                    };
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("  {} {}", icon, label), style,
                    ))));
                }
            }
        }

        let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };
        let list = List::new(items).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" mac-app-init "),
        );
        frame.render_widget(list, area);
    }

    fn render_content(&mut self, frame: &mut Frame, area: Rect) {
        if self.selected_tab == 0 {
            self.render_install(frame, area);
        } else {
            let domain_idx = self.selected_tab - 1;
            if let Some(Some(spec)) = self.specs.get(domain_idx) {
                self.render_domain(frame, area, spec);
            } else {
                self.render_no_spec(frame, area);
            }
        }
    }

    fn render_install(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(8)])
            .split(area);

        if self.available.is_empty() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled("  `mac` 바이너리를 찾을 수 없거나 사용 가능한 도메인이 없습니다.", Style::default().fg(Color::Yellow))),
                    Line::from(""),
                    Line::from(Span::styled("  터미널에서:  mac available", Style::default().fg(Color::Cyan))),
                ]).block(
                    Block::default().borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Install "),
                ),
                chunks[0],
            );
        } else {
            let items: Vec<ListItem> = self.available.iter().enumerate().map(|(i, name)| {
                let installed = self.is_installed(name);
                let marker = if installed { "[✓]" } else { "[ ]" };
                let status = if installed { "installed" } else { "available" };
                let (marker_style, name_style, status_style) = if installed {
                    (Style::default().fg(Color::Green).bold(), Style::default().fg(Color::White), Style::default().fg(Color::Green))
                } else {
                    (Style::default().fg(Color::DarkGray), Style::default().fg(Color::Gray), Style::default().fg(Color::DarkGray))
                };
                let row_style = if i == self.install_focus {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(marker, marker_style),
                    Span::raw(" "),
                    Span::styled(format!("{:<16}", name), name_style),
                    Span::styled(status, status_style),
                ])).style(row_style)
            }).collect();

            self.install_area_top = chunks[0].y + 1; // 박스 테두리 다음 줄부터 리스트

            frame.render_widget(
                List::new(items).block(
                    Block::default().borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(format!(" Install — {}/{} 설치됨 ", self.domains.len(), self.available.len())),
                ),
                chunks[0],
            );
        }

        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true }).block(
                Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Output — Enter/Space: 설치·삭제 토글 "),
            ),
            chunks[1],
        );
    }

    fn render_domain(&self, frame: &mut Frame, area: Rect, spec: &DomainSpec) {
        // Sections + Output area
        let constraints: Vec<Constraint> = spec.sections.iter()
            .map(|s| Constraint::Length(widgets::section_height(s)))
            .chain(std::iter::once(Constraint::Min(3)))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut button_section_offset = 0;
        for (i, section) in spec.sections.iter().enumerate() {
            // buttons 섹션만 focus_button 전달
            let focus = if matches!(section, Section::Buttons { .. }) {
                button_section_offset = i;
                self.focus_button
            } else { 0 };
            widgets::render_section(frame, chunks[i], section, focus);
        }
        let _ = button_section_offset;

        // Output
        let output_idx = spec.sections.len();
        if let Some(rect) = chunks.get(output_idx) {
            frame.render_widget(
                Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true }).block(
                    Block::default().borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Output "),
                ),
                *rect,
            );
        }
    }

    fn render_no_spec(&self, frame: &mut Frame, area: Rect) {
        let domain = self.domains.get(self.selected_tab.saturating_sub(1)).cloned().unwrap_or_default();
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(format!("  {} 도메인이 tui-spec을 지원하지 않습니다.", domain), Style::default().fg(Color::Yellow))),
                Line::from(""),
                Line::from(Span::styled("  터미널에서 직접 사용:", Style::default().fg(Color::Gray))),
                Line::from(Span::styled(format!("    mac run {} --help", domain), Style::default().fg(Color::Cyan))),
            ]).block(
                Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" {} ", domain)),
            ),
            area,
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press { return; }

        // 공통 키
        match key.code {
            KeyCode::Char('q') if key.modifiers.is_empty() => { self.should_quit = true; return; }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => { self.should_quit = true; return; }
            KeyCode::Char('r') => { self.load(); self.output = "Refreshed.".into(); return; }
            _ => {}
        }

        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::Content => self.handle_content_key(key),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            // 왼쪽 사이드바 클릭 → sidebar_entries 기반 매핑
            if mouse.column < 24 && mouse.row > 0 {
                let row = (mouse.row as usize).saturating_sub(1); // border 1줄 빼기
                if let Some(entry) = self.sidebar_entries.get(row).cloned() {
                    self.sidebar_cursor = row;
                    match entry {
                        SidebarEntry::Install => {
                            self.selected_tab = 0;
                            self.focus = Focus::Content;
                            self.focus_button = 0;
                        }
                        SidebarEntry::Domain { idx, .. } => {
                            self.selected_tab = idx + 1;
                            self.focus = Focus::Content;
                            self.focus_button = 0;
                        }
                        SidebarEntry::GroupHeader { group_id, .. } => {
                            if self.collapsed_groups.contains(&group_id) {
                                self.collapsed_groups.remove(&group_id);
                            } else {
                                self.collapsed_groups.insert(group_id);
                            }
                            self.build_sidebar();
                        }
                    }
                }
                return;
            }

            // Install 탭 리스트 클릭
            if self.selected_tab == 0 && mouse.column >= 24 && mouse.row >= self.install_area_top {
                let row_idx = (mouse.row - self.install_area_top) as usize;
                if row_idx < self.available.len() {
                    if self.install_focus == row_idx {
                        self.toggle_install();
                    } else {
                        self.install_focus = row_idx;
                    }
                }
            }
        }
    }

    fn current_buttons(&self) -> Option<(&str, &[crate::spec::Button])> {
        if self.selected_tab == 0 { return None; }
        let domain_idx = self.selected_tab - 1;
        let spec = self.specs.get(domain_idx)?.as_ref()?;
        for section in &spec.sections {
            if let Section::Buttons { items, .. } = section {
                return Some((&self.domains[domain_idx], items.as_slice()));
            }
        }
        None
    }

    fn activate_button(&mut self) {
        let (domain, command, args) = {
            let Some((domain, buttons)) = self.current_buttons() else { return; };
            let idx = self.focus_button.min(buttons.len().saturating_sub(1));
            let Some(b) = buttons.get(idx) else { return; };
            (domain.to_string(), b.command.clone(), b.args.clone())
        };
        self.output = format!("실행: {} {} {}\n", domain, command, args.join(" "));
        let result = run_action(&domain, &command, &args);
        self.output.push_str(&result);
        // 실행 후 spec 새로고침
        let domain_idx = self.selected_tab - 1;
        self.specs[domain_idx] = fetch_spec(&domain);
    }

    fn activate_by_key(&mut self, ch: char) {
        // 1) keybindings 우선 매치 (대소문자 구분)
        if self.activate_keybinding(ch) { return; }
        // 2) 버튼 key 매치 (legacy)
        let (domain, command, args) = {
            let Some((domain, buttons)) = self.current_buttons() else { return; };
            let Some(b) = buttons.iter().find(|b| b.key.as_deref() == Some(&ch.to_string())) else { return; };
            (domain.to_string(), b.command.clone(), b.args.clone())
        };
        self.output = format!("실행: {} {}\n", domain, command);
        let result = run_action(&domain, &command, &args);
        self.output.push_str(&result);
        let domain_idx = self.selected_tab - 1;
        self.specs[domain_idx] = fetch_spec(&domain);
    }

    /// keybindings 섹션에서 ch 와 일치하는 항목 실행. 성공 시 true.
    fn activate_keybinding(&mut self, ch: char) -> bool {
        if self.selected_tab == 0 { return false; }
        let domain_idx = self.selected_tab - 1;
        let Some(spec) = self.specs[domain_idx].as_ref() else { return false; };
        let key_str = ch.to_string();
        let kb = match spec.keybindings.iter().find(|k| k.key == key_str) {
            Some(k) => k.clone(),
            None => return false,
        };
        let domain = self.domains[domain_idx].clone();

        // 템플릿 치환
        let selected_data = self.selected_item_data();
        let args: Vec<String> = kb.args.iter().map(|a| self.resolve_template(a, &selected_data)).collect();

        // TODO: confirm modal (Step 2 예정). 현재는 바로 실행.
        self.output = format!("[{}] {} {}\n", kb.label, kb.command, args.join(" "));
        let result = run_action(&domain, &kb.command, &args);
        self.output.push_str(&result);

        if kb.reload {
            self.specs[domain_idx] = fetch_spec(&domain);
        }
        true
    }

    /// list_section 에서 현재 포커스된 항목의 data 맵 반환.
    /// v2 1단계: focus_button 을 리스트 인덱스로도 재활용 (단순화).
    fn selected_item_data(&self) -> std::collections::HashMap<String, String> {
        use std::collections::HashMap;
        let mut empty = HashMap::new();
        if self.selected_tab == 0 { return empty; }
        let domain_idx = self.selected_tab - 1;
        let Some(spec) = self.specs[domain_idx].as_ref() else { return empty; };
        let Some(list_title) = spec.list_section.as_ref() else { return empty; };
        for section in &spec.sections {
            if let crate::spec::Section::KeyValue { title, items } = section {
                if title != list_title { continue; }
                if items.is_empty() { return empty; }
                let idx = self.focus_button.min(items.len() - 1);
                let item = &items[idx];
                empty = item.data.clone();
                empty.entry("key".into()).or_insert(item.key.clone());
                empty.entry("value".into()).or_insert(item.value.clone());
                empty.entry("name".into()).or_insert(item.key.clone());
                return empty;
            }
        }
        empty
    }

    /// ${selected.<field>} 와 ${toggle:<field>} 치환.
    fn resolve_template(&self, template: &str, data: &std::collections::HashMap<String, String>) -> String {
        let mut out = String::new();
        let mut rest = template;
        while let Some(start) = rest.find("${") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let Some(end) = after.find('}') else {
                out.push_str("${"); out.push_str(after); break;
            };
            let expr = &after[..end];
            rest = &after[end + 1..];
            if let Some(field) = expr.strip_prefix("selected.") {
                out.push_str(data.get(field).map(|s| s.as_str()).unwrap_or(""));
            } else if let Some(field) = expr.strip_prefix("toggle:") {
                let cur = data.get(field).map(|s| s.as_str()).unwrap_or("false");
                out.push_str(if cur == "true" { "false" } else { "true" });
            } else {
                out.push_str("${"); out.push_str(expr); out.push('}');
            }
        }
        out.push_str(rest);
        out
    }
}
