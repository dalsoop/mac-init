//! App module — public facade + App struct composition.

mod async_ops;
mod domain_data;
mod input_modal;
mod render;
mod sidebar;
mod template;
mod types;

// ── Re-exports for tests and main.rs ──
pub use types::{ActiveView, DomainId, Focus, SidebarItem};
pub use input_modal::InputModal;

use crate::registry::{Registry, SystemRegistry};
use crate::spec::{DomainSpec, Section};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::cell::Cell;
use std::sync::Arc;
use tui_input::Input;

pub struct App {
    pub should_quit: bool,
    pub confirm_quit: bool,
    pub domains: Vec<String>,
    pub specs: Vec<Option<DomainSpec>>,
    pub available: Vec<String>,
    pub install_focus: usize,
    /// Computed during render, read by mouse handler. Interior mutability via Cell.
    pub(crate) install_area_top: Cell<u16>,
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
    pub bg_loading: Option<std::sync::mpsc::Receiver<(DomainId, Option<DomainSpec>)>>,
    /// 전체 프리로드 채널
    pub preload_rx: Option<std::sync::mpsc::Receiver<(DomainId, Option<DomainSpec>)>>,
    /// 백그라운드 액션 실행 결과 수신 (domain_id, reload, result)
    pub action_rx: Option<std::sync::mpsc::Receiver<(DomainId, bool, String)>>,
    /// 액션 실행 중 표시
    pub action_running: bool,
    /// 외부 의존 추상화 (프로세스, 파일시스템)
    reg: Arc<dyn Registry>,
    /// 텍스트 입력 모달
    pub input_modal: Option<InputModal>,
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
            install_area_top: Cell::new(0),
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

    // ── Active view helpers ──

    /// Returns the active view as an ActiveView enum.
    pub fn active_view(&self) -> ActiveView {
        if self.selected_tab == 0 {
            ActiveView::Install
        } else {
            ActiveView::Domain(DomainId(self.selected_tab - 1))
        }
    }

    /// Returns the currently active domain id, if any.
    pub fn active_domain_id(&self) -> Option<DomainId> {
        if self.selected_tab > 0 {
            Some(DomainId(self.selected_tab - 1))
        } else {
            None
        }
    }

    // ── Data loading ──

    pub fn load_fast(&mut self) {
        self.domains = self.reg.installed_domains();
        self.specs = vec![None; self.domains.len()];
        self.available = self.reg.available_domains();
        self.rebuild_sidebar();
    }

    pub fn load(&mut self) {
        self.domains = self.reg.installed_domains();
        self.specs = self.domains.iter().map(|d| self.reg.fetch_spec(d)).collect();
        self.available = self.reg.available_domains();
        self.rebuild_sidebar();
        if self.selected_tab > self.domains.len() { self.selected_tab = 0; }
    }

    fn rebuild_sidebar(&mut self) {
        self.sidebar_items = sidebar::build_sidebar(&self.domains, &self.specs);
        if self.sidebar_cursor >= self.sidebar_items.len() {
            self.sidebar_cursor = self.sidebar_items.len().saturating_sub(1);
        }
    }

    pub fn has_spec(&self, idx: usize) -> bool {
        self.specs.get(idx).and_then(|s| s.as_ref()).is_some()
    }

    pub(crate) fn is_installed(&self, name: &str) -> bool {
        self.domains.iter().any(|d| d == name)
    }

    /// 백그라운드에서 전체 도메인 spec 프리로드.
    pub fn preload_all_specs(&mut self) {
        let domains: Vec<(DomainId, String)> = self.domains.iter().enumerate()
            .map(|(i, d)| (DomainId(i), d.clone())).collect();
        self.preload_rx = Some(async_ops::spawn_preload_all(&domains, &self.reg));
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

    // ── Key handling ──

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press { return; }

        // 입력 모달이 활성화되어 있으면 모든 키를 모달로
        if let Some(modal) = &mut self.input_modal {
            match modal.handle_key(key) {
                input_modal::ModalAction::Submit { domain, command, args, .. } => {
                    self.input_modal = None;
                    let domain_id = self.active_domain_id().unwrap_or(DomainId(0));
                    self.output = format!("실행 중: {} {} {} …\n", domain, command, args.join(" "));
                    self.run_action_bg(domain_id, &domain, &command, &args, true);
                }
                input_modal::ModalAction::Cancel => {
                    self.input_modal = None;
                }
                input_modal::ModalAction::Consumed => {}
            }
            return;
        }

        // 공통 키
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('r') => {
                self.load();
                self.output = "Refreshed.".into();
                return;
            }
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

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => { self.confirm_quit = true; }
            KeyCode::Up | KeyCode::Char('k') => {
                self.sidebar_cursor = sidebar::sidebar_move(
                    &self.sidebar_items, self.sidebar_cursor, -1
                );
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.sidebar_cursor = sidebar::sidebar_move(
                    &self.sidebar_items, self.sidebar_cursor, 1
                );
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(item) = self.sidebar_items.get(self.sidebar_cursor).cloned() {
                    match item {
                        SidebarItem::Install => {
                            self.selected_tab = 0;
                            self.focus = Focus::Content;
                            self.content_section = 0;
                            self.focus_button = 0;
                        }
                        SidebarItem::Domain { id, .. } => {
                            self.selected_tab = id.0 + 1;
                            self.bg_loading = None;
                            self.content_section = 0;
                            self.focus_button = 0;
                            self.focus = Focus::SectionMenu;
                            // Trigger background spec load if missing
                            if id.0 < self.domains.len() && self.specs[id.0].is_none() {
                                self.pending_load = Some(id.0);
                            }
                        }
                        SidebarItem::GroupHeader(_) => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// 2열: 섹션 메뉴 키 처리.
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
                _ => {}
            }
        }
    }

    // ── Mouse handling ──

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            // 왼쪽 사이드바 클릭
            if mouse.column < 24 && mouse.row > 0 {
                let row = (mouse.row as usize).saturating_sub(1);
                if let Some(item) = self.sidebar_items.get(row).cloned() {
                    match item {
                        SidebarItem::Install => {
                            self.sidebar_cursor = row;
                            self.selected_tab = 0;
                            self.focus = Focus::Content;
                            self.content_section = 0;
                        }
                        SidebarItem::Domain { id, .. } => {
                            self.sidebar_cursor = row;
                            self.selected_tab = id.0 + 1;
                            self.bg_loading = None;
                            self.focus = Focus::SectionMenu;
                            self.content_section = 0;
                        }
                        SidebarItem::GroupHeader(_) => {}
                    }
                }
                return;
            }

            // Install 탭 리스트 클릭
            let iat = self.install_area_top.get();
            if self.selected_tab == 0 && mouse.column >= 24 && mouse.row >= iat {
                let row_idx = (mouse.row - iat) as usize;
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

    // ── Polling ──

    /// 메인 루프에서 호출 -- pending_load/bg_loading/preload 처리.
    pub fn poll_bg_loading(&mut self) {
        // pending_load: 백그라운드 스레드에서 spec 로드
        if let Some(idx) = self.pending_load.take() {
            if self.bg_loading.is_none() {
                let id = DomainId(idx);
                let domain = self.domains[idx].clone();
                self.bg_loading = Some(async_ops::spawn_spec_load(id, &domain, &self.reg));
            }
        }
        // 백그라운드 로드 완료 체크
        if let Some(ref rx) = self.bg_loading {
            if let Ok((id, spec)) = rx.try_recv() {
                self.specs[id.0] = spec;
                self.bg_loading = None;
            }
        }
        // 프리로드 결과 수신 (논블로킹, 하나씩)
        if let Some(ref rx) = self.preload_rx {
            while let Ok((id, spec)) = rx.try_recv() {
                if self.specs[id.0].is_none() {
                    self.specs[id.0] = spec;
                }
            }
        }
    }

    /// 메인 루프에서 호출 -- 백그라운드 액션 완료 확인.
    pub fn poll_action(&mut self) {
        let Some(ref rx) = self.action_rx else { return; };
        if let Ok((id, reload, result)) = rx.try_recv() {
            self.output.push_str(&result);
            if reload {
                if let Some(domain) = self.domains.get(id.0).cloned() {
                    self.specs[id.0] = self.reg.fetch_spec(&domain);
                }
            }
            self.action_rx = None;
            self.action_running = false;
        }
    }

    // ── Section helpers ──

    fn current_section_count(&self) -> usize {
        if self.selected_tab == 0 { return 1; }
        let idx = self.selected_tab - 1;
        self.specs.get(idx)
            .and_then(|s| s.as_ref())
            .map(|s| s.sections.len())
            .unwrap_or(1)
    }

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

    // ── Actions ──

    fn toggle_install(&mut self) {
        let Some(name) = self.available.get(self.install_focus).cloned() else { return; };
        let msg = if self.is_installed(&name) {
            format!("제거 중: {}...\n", name)
        } else {
            format!("설치 중: {}...\n", name)
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
        let domain_id = self.active_domain_id().unwrap_or(DomainId(0));
        self.output = format!("실행 중: {} {} {} …\n", domain, command, args.join(" "));
        self.run_action_bg(domain_id, &domain, &command, &args, true);
    }


    fn run_action_bg(&mut self, id: DomainId, domain: &str, command: &str, args: &[String], reload: bool) {
        self.action_rx = Some(async_ops::spawn_action(id, domain, command, args, reload, &self.reg));
        self.action_running = true;
    }

    // ── Edit modal ──

    fn open_edit_modal(&mut self) {
        if self.selected_tab == 0 { return; }
        let domain_idx = self.selected_tab - 1;
        let Some(spec) = self.specs[domain_idx].as_ref() else { return; };
        if spec.editables.is_empty() { return; }

        let section_idx = self.content_section.min(spec.sections.len().saturating_sub(1));
        let field_key = match spec.sections.get(section_idx) {
            Some(Section::KeyValue { items, .. }) if !items.is_empty() => {
                let idx = self.focus_button.min(items.len() - 1);
                items[idx].key.clone()
            }
            _ => return,
        };

        let Some(editable) = spec.editables.iter().find(|e| e.field == field_key) else { return; };

        let current_value = match spec.sections.get(section_idx) {
            Some(Section::KeyValue { items, .. }) => {
                let idx = self.focus_button.min(items.len() - 1);
                let v = &items[idx].value;
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

    // ── Template (public for test backward compatibility) ──

    pub fn resolve_template(&self, tmpl: &str, data: &std::collections::HashMap<String, String>) -> String {
        template::resolve_template(tmpl, data)
    }
}
