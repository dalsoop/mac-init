use crate::registry::{Registry, SystemRegistry};
use crate::spec::{DomainSpec, EditableField, Section};
use crate::widgets;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{prelude::*, widgets::*};
use std::sync::Arc;
use tui_input::Input;

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Focus {
    /// 1열: 도메인 목록
    Sidebar,
    /// 2열: 섹션 메뉴 (↑↓로 섹션 이동)
    SectionMenu,
    /// 3열: 선택 섹션 콘텐츠 (↑↓로 아이템 이동)
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
    /// 다음 틱에서 로드할 도메인 인덱스 (스피너 1프레임 보장)
    pub pending_load: Option<usize>,
    /// 단일 도메인 백그라운드 로딩
    pub bg_loading: Option<std::sync::mpsc::Receiver<(usize, Option<DomainSpec>)>>,
    /// 전체 프리로드 채널
    pub preload_rx: Option<std::sync::mpsc::Receiver<(usize, Option<DomainSpec>)>>,
    /// 백그라운드 액션 실행 결과 수신 (domain_idx, reload, result)
    pub action_rx: Option<std::sync::mpsc::Receiver<(usize, bool, String)>>,
    /// 액션 실행 중 표시
    pub action_running: bool,
    /// 외부 의존 추상화 (프로세스, 파일시스템)
    reg: Arc<dyn Registry>,
    /// 텍스트 입력 모달
    pub input_modal: Option<InputModal>,
}

/// 텍스트 입력 모달 상태.
pub struct InputModal {
    /// 모달 상단 라벨 (예: "Git 사용자 이름")
    pub label: String,
    /// 입력 버퍼
    pub input: Input,
    /// 확인 시 실행할 도메인
    pub domain: String,
    /// 확인 시 실행할 명령
    pub command: String,
    /// 인자 템플릿 (${value} → 사용자 입력으로 치환)
    pub args_template: Vec<String>,
}

#[derive(Clone)]
pub enum SidebarItem {
    GroupHeader(String),
    Install,
    Domain { idx: usize, label: String, icon: String },
}


impl App {
    pub fn new() -> Self {
        Self::with_registry(Arc::new(SystemRegistry))
    }

    pub fn with_registry(reg: Arc<dyn Registry>) -> Self {
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
            pending_load: None,
            bg_loading: None,
            preload_rx: None,
            action_rx: None,
            action_running: false,
            reg,
            input_modal: None,
        }
    }

    pub fn load_fast(&mut self) {
        self.domains = self.reg.installed_domains();
        self.specs = vec![None; self.domains.len()];
        self.available = self.reg.available_domains();
        self.build_sidebar();
    }

    pub fn load(&mut self) {
        self.domains = self.reg.installed_domains();
        self.specs = self.domains.iter().map(|d| self.reg.fetch_spec(d)).collect();
        self.available = self.reg.available_domains();
        self.build_sidebar();
        if self.selected_tab > self.domains.len() { self.selected_tab = 0; }
    }

    /// 선택된 도메인 spec 로드 (lazy + 캐시)
    /// 한 번 로드하면 refresh 전까지 재호출 안 함.
    pub fn ensure_spec(&mut self, idx: usize) {
        if self.specs[idx].is_none() {
            self.specs[idx] = self.reg.fetch_spec(&self.domains[idx]);
        }
    }

    pub fn has_spec(&self, idx: usize) -> bool {
        self.specs.get(idx).and_then(|s| s.as_ref()).is_some()
    }

    /// 백그라운드에서 전체 도메인 spec 프리로드. 사이드바는 즉시 표시.
    pub fn preload_all_specs(&mut self) {
        use std::sync::mpsc;
        use std::thread;

        let domains: Vec<(usize, String)> = self.domains.iter().enumerate()
            .map(|(i, d)| (i, d.clone())).collect();
        let (tx, rx) = mpsc::channel();
        let reg = Arc::clone(&self.reg);

        thread::spawn(move || {
            for (idx, domain) in domains {
                let spec = reg.fetch_spec(&domain);
                let _ = tx.send((idx, spec));
            }
        });

        self.preload_rx = Some(rx);
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
        if let Some(domain) = self.domains.get(idx).cloned() {
            self.specs[idx] = self.reg.fetch_spec(&domain);
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => { self.confirm_quit = true; }
            KeyCode::Up | KeyCode::Char('k') => { self.sidebar_move(-1); }
            KeyCode::Down | KeyCode::Char('j') => { self.sidebar_move(1); }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                // 선택 → 섹션 메뉴 또는 콘텐츠로
                if let Some(item) = self.sidebar_items.get(self.sidebar_cursor).cloned() {
                    match item {
                        SidebarItem::Install => {
                            self.selected_tab = 0;
                            self.focus = Focus::Content; self.content_section = 0;
                            self.focus_button = 0;
                        }
                        SidebarItem::Domain { idx, .. } => {
                            self.selected_tab = idx + 1;
                            self.bg_loading = None;
                            self.content_section = 0;
                            self.focus_button = 0;
                            self.focus = Focus::SectionMenu;
                        }
                        SidebarItem::GroupHeader(_) => {} // 선택 불가
                    }
                }
            }
            _ => {}
        }
    }

    /// 커서 이동 — 그룹 헤더 skip. 선택 가능 항목 없으면 움직이지 않음.
    fn sidebar_move(&mut self, dir: i32) {
        let len = self.sidebar_items.len();
        if len == 0 { return; }
        let has_selectable = self.sidebar_items.iter().any(|i| !matches!(i, SidebarItem::GroupHeader(_)));
        if !has_selectable { return; }
        let mut next = self.sidebar_cursor as i32 + dir;
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

    /// 2열: 섹션 메뉴 키 처리. ↑↓로 섹션 이동, Enter/→로 콘텐츠 진입.
    fn handle_section_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.focus = Focus::Sidebar;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let max = self.current_section_count();
                if max > 0 {
                    self.content_section = (self.content_section + max - 1) % max;
                    self.focus_button = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.current_section_count();
                if max > 0 {
                    self.content_section = (self.content_section + 1) % max;
                    self.focus_button = 0;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.focus = Focus::Content;
                self.focus_button = 0;
            }
            KeyCode::Tab => {
                let max = self.current_section_count();
                if max > 0 {
                    self.content_section = (self.content_section + 1) % max;
                    self.focus_button = 0;
                }
            }
            KeyCode::BackTab => {
                let max = self.current_section_count();
                if max > 0 {
                    self.content_section = (self.content_section + max - 1) % max;
                    self.focus_button = 0;
                }
            }
            _ => {}
        }
    }

    fn handle_content_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.focus = Focus::SectionMenu;
                return;
            }
            KeyCode::Tab => {
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
                    let max = self.current_section_item_count();
                    if max > 0 {
                        self.focus_button = (self.focus_button + 1).min(max - 1);
                    }
                }
                KeyCode::Enter => self.activate_button(),
                KeyCode::Char('e') => self.open_edit_modal(),
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

    /// 현재 선택된 섹션의 아이템 수 (KV items, Table rows, Buttons, Text lines).
    fn current_section_item_count(&self) -> usize {
        if self.selected_tab == 0 { return 0; }
        let idx = self.selected_tab - 1;
        let Some(Some(spec)) = self.specs.get(idx) else { return 0; };
        let section_idx = self.content_section.min(spec.sections.len().saturating_sub(1));
        match spec.sections.get(section_idx) {
            Some(Section::KeyValue { items, .. }) => items.len(),
            Some(Section::Table { rows, .. }) => rows.len(),
            Some(Section::Buttons { items, .. }) => items.len(),
            Some(Section::Text { content, .. }) => content.lines().count(),
            None => 0,
        }
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
            self.reg.remove_domain(&name)
        } else {
            self.reg.install_domain(&name)
        };
        self.output.push_str(&result);
        self.load();
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(frame.area());

        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(0)])
            .split(outer[0]);

        self.render_sidebar(frame, main[0]);
        self.render_content(frame, main[1]);

        // 하단 키 안내
        let hints = if self.input_modal.is_some() {
            "Enter 확인 │ Esc 취소"
        } else {
            match self.focus {
                Focus::Sidebar => "↑↓ 이동 │ Enter/→ 선택 │ r 새로고침 │ Esc 종료",
                Focus::SectionMenu => "↑↓ 섹션 │ Enter/→ 콘텐츠 │ ←/Esc 뒤로 │ r 새로고침",
                Focus::Content => "↑↓ 항목 │ e 수정 │ Tab 섹션 │ ←/Esc 뒤로 │ Enter 실행",
            }
        };
        frame.render_widget(
            Paragraph::new(Span::styled(hints, Style::default().fg(Color::DarkGray))),
            outer[1],
        );

        // 입력 모달 오버레이
        self.render_modal(frame);
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
                        Style::default().bg(Color::Cyan).fg(Color::Black).bold()
                    } else if selected {
                        Style::default().fg(Color::Cyan).bold()
                    } else { Style::default().fg(Color::White) };
                    items.push(ListItem::new(Line::from(Span::styled("   📋 도메인 현황", style))));
                }
                SidebarItem::Domain { idx, label, icon } => {
                    let selected = self.selected_tab == idx + 1;
                    let style = if is_cursor {
                        Style::default().bg(Color::Cyan).fg(Color::Black).bold()
                    } else if selected {
                        Style::default().fg(Color::Cyan).bold()
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
            return;
        }
        let domain_idx = self.selected_tab - 1;
        if self.specs.get(domain_idx).and_then(|s| s.as_ref()).is_none() {
            self.render_no_spec(frame, area);
            if domain_idx < self.domains.len() && self.specs[domain_idx].is_none() {
                self.pending_load = Some(domain_idx);
            }
            return;
        }

        // 2열(서브메뉴) + 3열(섹션 콘텐츠)
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(area);

        self.render_section_menu(frame, cols[0]);
        self.render_section_content(frame, cols[1]);
    }

    /// 2열: 섹션 제목 리스트
    fn render_section_menu(&self, frame: &mut Frame, area: Rect) {
        let domain_idx = self.selected_tab - 1;
        let Some(Some(spec)) = self.specs.get(domain_idx) else { return; };
        let menu_focused = self.focus == Focus::SectionMenu;
        let content_focused = self.focus == Focus::Content;

        let mut items = Vec::new();
        for (i, section) in spec.sections.iter().enumerate() {
            let title = match section {
                Section::KeyValue { title, .. } => title,
                Section::Table { title, .. } => title,
                Section::Buttons { title, .. } => title,
                Section::Text { title, .. } => title,
            };
            let is_selected = i == self.content_section;
            let style = if (menu_focused || content_focused) && is_selected {
                Style::default().bg(Color::Cyan).fg(Color::Black).bold()
            } else {
                Style::default().fg(Color::White)
            };
            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {}", title), style,
            ))));
        }

        let border = if menu_focused { Color::Cyan } else { Color::DarkGray };
        let domain_label = spec.tab.label_ko.as_deref().unwrap_or(&spec.tab.label);
        let list = List::new(items).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(format!(" {} ", domain_label)),
        );
        frame.render_widget(list, area);
    }

    /// 3열: 선택된 섹션 내용만
    fn render_section_content(&mut self, frame: &mut Frame, area: Rect) {
        let domain_idx = self.selected_tab - 1;
        let Some(Some(spec)) = self.specs.get(domain_idx) else { return; };
        let section_idx = self.content_section.min(spec.sections.len().saturating_sub(1));
        let Some(section) = spec.sections.get(section_idx) else { return; };

        let domain = &self.domains[domain_idx];

        // output 영역 분할
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(6)])
            .split(area);

        // 섹션 렌더
        let content_focused = self.focus == Focus::Content;
        widgets::render_section(frame, chunks[0], section, self.focus_button, content_focused);

        // output
        if !self.output.is_empty() {
            let title = if self.action_running {
                let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                static ACTION_START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                let tick = (ACTION_START.get_or_init(std::time::Instant::now).elapsed().as_millis() / 66) as usize;
                format!(" {} 실행 중… ", spinners[tick % spinners.len()])
            } else {
                " Output ".to_string()
            };
            frame.render_widget(
                Paragraph::new(self.output.as_str())
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .block(Block::default().borders(Borders::ALL)
                        .border_style(Style::default().fg(if self.action_running { Color::Yellow } else { Color::DarkGray }))
                        .title(title)),
                chunks[1],
            );
        } else {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!("  mac run {} --help", domain),
                    Style::default().fg(Color::DarkGray),
                )).block(Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))),
                chunks[1],
            );
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
                // 설치된 도메인은 usage 정보로 사용 상태 표시
                let (marker, status, marker_style, name_style, status_style) = if installed {
                    let domain_idx = self.domains.iter().position(|d| d == name);
                    let usage = domain_idx.and_then(|idx| {
                        self.specs.get(idx).and_then(|s| s.as_ref()).and_then(|s| s.usage.as_ref())
                    });
                    match usage {
                        Some(u) if u.active => (
                            "✓ 사용",
                            u.summary.clone().unwrap_or_default(),
                            Style::default().fg(Color::Green).bold(),
                            Style::default().fg(Color::White),
                            Style::default().fg(Color::Green),
                        ),
                        Some(u) => (
                            "○ 미사용",
                            u.summary.clone().unwrap_or_default(),
                            Style::default().fg(Color::Yellow),
                            Style::default().fg(Color::White),
                            Style::default().fg(Color::Yellow),
                        ),
                        None => (
                            "✓ 설치됨",
                            "확인 중…".to_string(),
                            Style::default().fg(Color::Cyan),
                            Style::default().fg(Color::White),
                            Style::default().fg(Color::DarkGray),
                        ),
                    }
                } else {
                    (
                        "  미설치",
                        String::new(),
                        Style::default().fg(Color::DarkGray),
                        Style::default().fg(Color::Gray),
                        Style::default().fg(Color::DarkGray),
                    )
                };
                let row_style = if i == self.install_focus {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                let mut spans = vec![
                    Span::raw(" "),
                    Span::styled(format!("{:<10}", marker), marker_style),
                    Span::styled(format!("{:<14}", name), name_style),
                ];
                if !status.is_empty() {
                    spans.push(Span::styled(status, status_style));
                }
                ListItem::new(Line::from(spans)).style(row_style)
            }).collect();

            self.install_area_top = chunks[0].y + 1; // 박스 테두리 다음 줄부터 리스트

            frame.render_widget(
                List::new(items).block(
                    Block::default().borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(format!(" 도메인 현황 — {}/{} 설치됨 ", self.domains.len(), self.available.len())),
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
            widgets::render_section(frame, chunks[i], section, focus, self.focus == Focus::Content);
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
        let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        // Instant 기반 (SystemTime 대신 — 클럭 점프 방지)
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        let elapsed = START.get_or_init(std::time::Instant::now).elapsed();
        let tick = (elapsed.as_millis() / 66) as usize;
        let spinner = spinners[tick % spinners.len()];
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {} 로딩 중...", spinner),
                    Style::default().fg(Color::Cyan).bold(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {} 도메인 정보를 가져오고 있습니다.", domain),
                    Style::default().fg(Color::DarkGray),
                )),
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

        // 입력 모달이 활성화되어 있으면 모든 키를 모달로
        if self.input_modal.is_some() {
            self.handle_modal_key(key);
            return;
        }

        // 공통 키
        match key.code {
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
            Focus::SectionMenu => self.handle_section_menu_key(key),
            Focus::Content => self.handle_content_key(key),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            // 왼쪽 사이드바 클릭
            if mouse.column < 24 && mouse.row > 0 {
                let row = (mouse.row as usize).saturating_sub(1); // border 1줄 빼기
                if let Some(item) = self.sidebar_items.get(row).cloned() {
                    match item {
                        SidebarItem::Install => {
                            self.sidebar_cursor = row;
                            self.selected_tab = 0;
                            self.focus = Focus::Content; self.content_section = 0;
                        }
                        SidebarItem::Domain { idx, .. } => {
                            self.sidebar_cursor = row;
                            self.selected_tab = idx + 1;
                            self.bg_loading = None;
                            self.focus = Focus::SectionMenu; self.content_section = 0;
                        }
                        SidebarItem::GroupHeader(_) => {} // 커서 안 옮김
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
        if self.action_running { return; }
        let (domain, command, args) = {
            let Some((domain, buttons)) = self.current_buttons() else { return; };
            let idx = self.focus_button.min(buttons.len().saturating_sub(1));
            let Some(b) = buttons.get(idx) else { return; };
            (domain.to_string(), b.command.clone(), b.args.clone())
        };
        let domain_idx = self.selected_tab - 1;
        self.output = format!("실행 중: {} {} {} …\n", domain, command, args.join(" "));
        self.run_action_bg(domain_idx, &domain, &command, &args, true);
    }

    fn activate_by_key(&mut self, ch: char) {
        if self.action_running { return; }
        // 1) keybindings 우선 매치 (대소문자 구분)
        if self.activate_keybinding(ch) { return; }
        // 2) 버튼 key 매치 (legacy)
        let (domain, command, args) = {
            let Some((domain, buttons)) = self.current_buttons() else { return; };
            let Some(b) = buttons.iter().find(|b| b.key.as_deref() == Some(&ch.to_string())) else { return; };
            (domain.to_string(), b.command.clone(), b.args.clone())
        };
        let domain_idx = self.selected_tab - 1;
        self.output = format!("실행 중: {} {} …\n", domain, command);
        self.run_action_bg(domain_idx, &domain, &command, &args, true);
    }

    /// keybindings 섹션에서 ch 와 일치하는 항목 실행. 성공 시 true.
    fn activate_keybinding(&mut self, ch: char) -> bool {
        if self.selected_tab == 0 { return false; }
        if self.action_running { return false; }
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

        self.output = format!("[{}] 실행 중: {} {} …\n", kb.label, kb.command, args.join(" "));
        self.run_action_bg(domain_idx, &domain, &kb.command, &args, kb.reload);
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
    pub fn resolve_template(&self, template: &str, data: &std::collections::HashMap<String, String>) -> String {
        let mut out = String::new();
        let mut rest = template;
        while let Some(start) = rest.find("${") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let Some(end) = after.find('}') else {
                out.push_str("${"); out.push_str(after);
                rest = "";
                break;
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

    /// 액션을 백그라운드 스레드에서 실행. TUI는 멈추지 않음.
    fn run_action_bg(&mut self, domain_idx: usize, domain: &str, command: &str, args: &[String], reload: bool) {
        let (tx, rx) = std::sync::mpsc::channel();
        let domain = domain.to_string();
        let command = command.to_string();
        let args = args.to_vec();
        let reg = Arc::clone(&self.reg);
        std::thread::spawn(move || {
            let result = reg.run_action(&domain, &command, &args);
            let _ = tx.send((domain_idx, reload, result));
        });
        self.action_rx = Some(rx);
        self.action_running = true;
    }

    /// 메인 루프에서 호출 — pending_load/bg_loading/preload 처리.
    pub fn poll_bg_loading(&mut self) {
        // pending_load: 백그라운드 스레드에서 spec 로드
        if let Some(idx) = self.pending_load.take() {
            if self.bg_loading.is_none() {
                let domain = self.domains[idx].clone();
                let (tx, rx) = std::sync::mpsc::channel();
                let reg = Arc::clone(&self.reg);
                std::thread::spawn(move || {
                    let spec = reg.fetch_spec(&domain);
                    let _ = tx.send((idx, spec));
                });
                self.bg_loading = Some(rx);
            }
        }
        // 백그라운드 로드 완료 체크
        if let Some(ref rx) = self.bg_loading {
            if let Ok((idx, spec)) = rx.try_recv() {
                self.specs[idx] = spec;
                self.bg_loading = None;
            }
        }
        // 프리로드 결과 수신 (논블로킹, 하나씩)
        if let Some(ref rx) = self.preload_rx {
            while let Ok((idx, spec)) = rx.try_recv() {
                if self.specs[idx].is_none() {
                    self.specs[idx] = spec;
                }
            }
        }
    }

    /// 메인 루프에서 호출 — 백그라운드 액션 완료 확인.
    pub fn poll_action(&mut self) {
        let Some(ref rx) = self.action_rx else { return; };
        if let Ok((domain_idx, reload, result)) = rx.try_recv() {
            self.output.push_str(&result);
            if reload {
                if let Some(domain) = self.domains.get(domain_idx).cloned() {
                    self.specs[domain_idx] = self.reg.fetch_spec(&domain);
                }
            }
            self.action_rx = None;
            self.action_running = false;
        }
    }

    // ── 입력 모달 ──

    /// 현재 포커스된 KV 항목에 대해 편집 가능 필드가 있으면 모달을 연다.
    fn open_edit_modal(&mut self) {
        if self.selected_tab == 0 { return; }
        let domain_idx = self.selected_tab - 1;
        let Some(spec) = self.specs[domain_idx].as_ref() else { return; };
        if spec.editables.is_empty() { return; }

        // 현재 섹션의 현재 포커스 항목의 key를 가져옴
        let section_idx = self.content_section.min(spec.sections.len().saturating_sub(1));
        let field_key = match spec.sections.get(section_idx) {
            Some(Section::KeyValue { items, .. }) if !items.is_empty() => {
                let idx = self.focus_button.min(items.len() - 1);
                items[idx].key.clone()
            }
            _ => return,
        };

        // editables에서 이 field에 매칭되는 정의 찾기
        let Some(editable) = spec.editables.iter().find(|e| e.field == field_key) else { return; };

        // 현재 값을 pre-fill
        let current_value = match spec.sections.get(section_idx) {
            Some(Section::KeyValue { items, .. }) => {
                let idx = self.focus_button.min(items.len() - 1);
                let v = &items[idx].value;
                // "✓ value" 형태면 "value" 부분만 추출
                v.strip_prefix("✓ ").or(v.strip_prefix("✗ ")).unwrap_or(v).to_string()
            }
            _ => String::new(),
        };

        self.input_modal = Some(InputModal {
            label: editable.label.clone(),
            input: Input::new(current_value),
            domain: self.domains[domain_idx].clone(),
            command: editable.command.clone(),
            args_template: editable.args.clone(),
        });
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        let Some(modal) = &mut self.input_modal else { return; };
        match key.code {
            KeyCode::Enter => {
                let value = modal.input.value().to_string();
                let domain = modal.domain.clone();
                let command = modal.command.clone();
                let args: Vec<String> = modal.args_template.iter()
                    .map(|a| a.replace("${value}", &value))
                    .collect();
                self.input_modal = None;

                let domain_idx = self.selected_tab.saturating_sub(1);
                self.output = format!("실행 중: {} {} {} …\n", domain, command, args.join(" "));
                self.run_action_bg(domain_idx, &domain, &command, &args, true);
            }
            KeyCode::Esc => {
                self.input_modal = None;
            }
            KeyCode::Char(c) => {
                modal.input = modal.input.clone().with_value(
                    format!("{}{}", modal.input.value(), c)
                );
            }
            KeyCode::Backspace => {
                let v = modal.input.value().to_string();
                if !v.is_empty() {
                    let new_v: String = v.chars().take(v.chars().count() - 1).collect();
                    modal.input = modal.input.clone().with_value(new_v);
                }
            }
            _ => {}
        }
    }

    /// 입력 모달 렌더링 (오버레이).
    pub fn render_modal(&self, frame: &mut Frame) {
        let Some(modal) = &self.input_modal else { return; };
        let area = frame.area();
        let w = 60.min(area.width.saturating_sub(4));
        let h = 3;
        let modal_area = Rect {
            x: (area.width.saturating_sub(w)) / 2,
            y: (area.height.saturating_sub(h)) / 2,
            width: w,
            height: h,
        };

        // 배경 지우기
        frame.render_widget(Clear, modal_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" {} ", modal.label));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        let scroll = modal.input.visual_scroll(inner.width as usize);
        let input_widget = Paragraph::new(modal.input.value())
            .scroll((0, scroll as u16))
            .style(Style::default().fg(Color::White));
        frame.render_widget(input_widget, inner);

        // 커서 위치
        frame.set_cursor_position((
            inner.x + (modal.input.visual_cursor().saturating_sub(scroll)) as u16,
            inner.y,
        ));
    }
}
