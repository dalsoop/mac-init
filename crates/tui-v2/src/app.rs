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

#[derive(PartialEq, Clone, Copy)]
pub enum Focus {
    /// 1열: 도메인 목록
    Sidebar,
    /// 2열: 선택 도메인 콘텐츠
    Content,
}

pub struct App {
    pub should_quit: bool,
    pub confirm_quit: bool,
    pub domains: Vec<String>,
    pub specs: Vec<Option<DomainSpec>>,
    pub available: Vec<String>,
    pub install_focus: usize,
    pub install_area_top: u16,
    pub selected_tab: usize,
    pub focus_button: usize,
    pub content_section: usize,
    pub output: String,
    /// 1열 항목: 그룹 헤더 + 도메인 flat 목록
    pub sidebar_items: Vec<SidebarItem>,
    /// 1열 커서 (선택 가능 항목만 이동)
    pub sidebar_cursor: usize,
    pub focus: Focus,
}

#[derive(Clone)]
pub enum SidebarItem {
    GroupHeader(String),
    Install,
    Domain { idx: usize, label: String, icon: String },
}


impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            confirm_quit: false,
            domains: Vec::new(),
            specs: Vec::new(),
            available: Vec::new(),
            install_focus: 0,
            install_area_top: 0,
            selected_tab: 0,
            focus_button: 0,
            content_section: 0,
            output: String::new(),
            sidebar_items: Vec::new(),
            sidebar_cursor: 0,
            focus: Focus::Sidebar,
        }
    }

    pub fn load_fast(&mut self) {
        self.domains = installed_domains();
        self.specs = vec![None; self.domains.len()];
        self.available = available_domains();
        self.build_sidebar();
    }

    pub fn load(&mut self) {
        self.domains = installed_domains();
        self.specs = self.domains.iter().map(|d| fetch_spec(d)).collect();
        self.available = available_domains();
        self.build_sidebar();
        if self.selected_tab > self.domains.len() { self.selected_tab = 0; }
    }

    /// 선택된 도메인 spec 로드 (lazy)
    fn ensure_spec(&mut self, idx: usize) {
        if self.specs[idx].is_none() {
            self.specs[idx] = fetch_spec(&self.domains[idx]);
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
            KeyCode::Esc => { self.confirm_quit = true; }
            KeyCode::Up | KeyCode::Char('k') => { self.sidebar_move(-1); }
            KeyCode::Down | KeyCode::Char('j') => { self.sidebar_move(1); }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                // 선택 → 콘텐츠로
                if let Some(item) = self.sidebar_items.get(self.sidebar_cursor).cloned() {
                    match item {
                        SidebarItem::Install => {
                            self.selected_tab = 0;
                            self.focus = Focus::Content; self.content_section = 0;
                            self.focus_button = 0;
                        }
                        SidebarItem::Domain { idx, .. } => {
                            self.selected_tab = idx + 1;
                            self.output = "로딩 중...".into();
                            self.ensure_spec(idx);
                            self.output = String::new();
                            self.focus = Focus::Content; self.content_section = 0;
                            self.focus_button = 0;
                        }
                        SidebarItem::GroupHeader(_) => {} // 선택 불가
                    }
                }
            }
            _ => {}
        }
    }

    /// 커서 이동 — 그룹 헤더 skip
    fn sidebar_move(&mut self, dir: i32) {
        let len = self.sidebar_items.len();
        if len == 0 { return; }
        let mut next = self.sidebar_cursor as i32 + dir;
        // wrap + skip headers
        for _ in 0..len {
            if next < 0 { next = len as i32 - 1; }
            if next >= len as i32 { next = 0; }
            if !matches!(self.sidebar_items[next as usize], SidebarItem::GroupHeader(_)) {
                break;
            }
            next += dir;
        }
        self.sidebar_cursor = next.clamp(0, len as i32 - 1) as usize;
    }

    fn handle_content_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.focus = Focus::Sidebar;
                return;
            }
            KeyCode::Tab => {
                // 콘텐츠 내 섹션 이동 (다음)
                let max = self.current_section_count();
                if max > 0 {
                    self.content_section = (self.content_section + 1) % max;
                    self.focus_button = 0;
                }
                return;
            }
            KeyCode::BackTab => {
                let max = self.current_section_count();
                if max > 0 {
                    self.content_section = (self.content_section + max - 1) % max;
                    self.focus_button = 0;
                }
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

    fn current_section_count(&self) -> usize {
        if self.selected_tab == 0 { return 1; }
        let idx = self.selected_tab - 1;
        self.specs.get(idx)
            .and_then(|s| s.as_ref())
            .map(|s| s.sections.len())
            .unwrap_or(1)
    }

    fn build_sidebar(&mut self) {
        let mut items = Vec::new();
        for &(group_id, group_label) in GROUPS {
            let is_init = group_id == "init";
            let mut has_domains = false;
            let mut group_domains = Vec::new();
            for (i, domain) in self.domains.iter().enumerate() {
                let spec_group = self.specs[i].as_ref()
                    .and_then(|s| s.group.as_deref())
                    .unwrap_or_else(|| default_group(domain));
                if spec_group != group_id { continue; }
                let label = self.specs[i].as_ref()
                    .map(|s| s.tab.label_ko.as_deref().unwrap_or(&s.tab.label).to_string())
                    .unwrap_or_else(|| domain.clone());
                let icon = self.specs[i].as_ref()
                    .and_then(|s| s.tab.icon.clone())
                    .unwrap_or_default();
                group_domains.push((i, label, icon));
                has_domains = true;
            }
            if !is_init && !has_domains { continue; }
            items.push(SidebarItem::GroupHeader(group_label.to_string()));
            if is_init { items.push(SidebarItem::Install); }
            for (idx, label, icon) in group_domains {
                items.push(SidebarItem::Domain { idx, label, icon });
            }
        }
        self.sidebar_items = items;
        if self.sidebar_cursor >= self.sidebar_items.len() {
            self.sidebar_cursor = self.sidebar_items.len().saturating_sub(1);
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
        let focused = self.focus == Focus::Sidebar;
        let mut items = Vec::new();

        for (i, item) in self.sidebar_items.iter().enumerate() {
            let is_cursor = focused && i == self.sidebar_cursor;
            match item {
                SidebarItem::GroupHeader(label) => {
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!(" {}", label),
                        Style::default().fg(Color::DarkGray).bold(),
                    ))));
                }
                SidebarItem::Install => {
                    let selected = self.selected_tab == 0;
                    let style = if is_cursor {
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold()
                    } else if selected {
                        Style::default().bg(Color::Cyan).fg(Color::Black).bold()
                    } else { Style::default().fg(Color::White) };
                    items.push(ListItem::new(Line::from(Span::styled("   📥 설치 관리", style))));
                }
                SidebarItem::Domain { idx, label, icon } => {
                    let selected = self.selected_tab == idx + 1;
                    let style = if is_cursor {
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold()
                    } else if selected {
                        Style::default().bg(Color::Cyan).fg(Color::Black).bold()
                    } else { Style::default().fg(Color::White) };
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("   {} {}", icon, label), style,
                    ))));
                }
            }
        }

        if self.confirm_quit {
            items.push(ListItem::new(Line::from("")));
            items.push(ListItem::new(Line::from(Span::styled(
                " 종료? (y/n)", Style::default().fg(Color::Red).bold(),
            ))));
        }

        let border = if focused { Color::Cyan } else { Color::DarkGray };
        let list = List::new(items).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(border))
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
                Line::from(Span::styled("  로딩 중...", Style::default().fg(Color::Cyan).bold())),
                Line::from(""),
                Line::from(Span::styled(format!("  {} 도메인 정보를 가져오고 있습니다.", domain), Style::default().fg(Color::DarkGray))),
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

        // 종료 확인 모드
        if self.confirm_quit {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => { self.should_quit = true; }
                _ => { self.confirm_quit = false; }
            }
            return;
        }

        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::Content => self.handle_content_key(key),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            // 왼쪽 사이드바 클릭
            if mouse.column < 24 && mouse.row > 0 {
                let row = (mouse.row as usize).saturating_sub(1); // border 1줄 빼기
                if row < self.sidebar_items.len() {
                    self.sidebar_cursor = row;
                    if let Some(item) = self.sidebar_items.get(row).cloned() {
                        match item {
                            SidebarItem::Install => {
                                self.selected_tab = 0;
                                self.focus = Focus::Content; self.content_section = 0;
                            }
                            SidebarItem::Domain { idx, .. } => {
                                self.ensure_spec(idx);
                                self.selected_tab = idx + 1;
                                self.focus = Focus::Content; self.content_section = 0;
                            }
                            SidebarItem::GroupHeader(_) => {}
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
