use crate::registry::{fetch_spec, installed_domains, run_action};
use crate::spec::{DomainSpec, Section};
use crate::widgets;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{prelude::*, widgets::*};

pub struct App {
    pub should_quit: bool,
    pub domains: Vec<String>,     // 설치된 도메인 이름
    pub specs: Vec<Option<DomainSpec>>,
    pub selected_tab: usize,      // 0 = Install, 1+ = 도메인
    pub focus_button: usize,      // Buttons 섹션 내 포커스
    pub output: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            domains: Vec::new(),
            specs: Vec::new(),
            selected_tab: 0,
            focus_button: 0,
            output: String::new(),
        }
    }

    pub fn load(&mut self) {
        self.domains = installed_domains();
        self.specs = self.domains.iter().map(|d| fetch_spec(d)).collect();
        if self.selected_tab > self.domains.len() {
            self.selected_tab = 0;
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(frame.area());

        self.render_sidebar(frame, chunks[0]);
        self.render_content(frame, chunks[1]);
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        // Install (항상 첫 번째)
        let style = if self.selected_tab == 0 {
            Style::default().bg(Color::Cyan).fg(Color::Black).bold()
        } else {
            Style::default().fg(Color::White)
        };
        items.push(ListItem::new(Line::from(Span::styled(" Install ", style))));

        if !self.domains.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled("─────────", Style::default().fg(Color::DarkGray)))));
        }

        // Installed domains
        for (i, domain) in self.domains.iter().enumerate() {
            let idx = i + 1;
            let label = match &self.specs[i] {
                Some(spec) => spec.tab.label.clone(),
                None => domain.clone(),
            };
            let style = if self.selected_tab == idx {
                Style::default().bg(Color::Cyan).fg(Color::Black).bold()
            } else {
                Style::default().fg(Color::White)
            };
            items.push(ListItem::new(Line::from(Span::styled(format!(" {} ", label), style))));
        }

        let list = List::new(items).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" mac "),
        );
        frame.render_widget(list, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
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

    fn render_install(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(6)])
            .split(area);

        let text = if self.domains.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled("  설치된 도메인이 없습니다.", Style::default().fg(Color::Yellow))),
                Line::from(""),
                Line::from(Span::styled("  터미널에서:", Style::default().fg(Color::Gray))),
                Line::from(Span::styled("    mac available            # 사용 가능한 도메인", Style::default().fg(Color::Cyan))),
                Line::from(Span::styled("    mac install keyboard     # 도메인 설치", Style::default().fg(Color::Cyan))),
                Line::from(""),
                Line::from(Span::styled("  설치하면 왼쪽 사이드바에 탭이 자동 생성됩니다.", Style::default().fg(Color::Gray))),
            ]
        } else {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(format!("  설치된 도메인: {}개", self.domains.len()), Style::default().fg(Color::Green))),
                Line::from(""),
            ];
            for d in &self.domains {
                lines.push(Line::from(vec![
                    Span::raw("  ✓ "),
                    Span::styled(d.clone(), Style::default().fg(Color::White)),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  j/k 또는 마우스로 왼쪽 탭 선택", Style::default().fg(Color::Gray))));
            lines
        };

        frame.render_widget(
            Paragraph::new(text).block(
                Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Install "),
            ),
            chunks[0],
        );

        frame.render_widget(
            Paragraph::new(self.output.as_str()).wrap(Wrap { trim: true }).block(
                Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Output "),
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

        match key.code {
            KeyCode::Char('q') if key.modifiers.is_empty() => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_tab > 0 { self.selected_tab -= 1; self.focus_button = 0; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_tab < self.domains.len() { self.selected_tab += 1; self.focus_button = 0; }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                // 버튼 포커스 이전
                self.focus_button = self.focus_button.saturating_sub(1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.focus_button = self.focus_button.saturating_add(1);
                // clamp below in activate
            }
            KeyCode::Enter => self.activate_button(),
            KeyCode::Char('r') => { self.load(); self.output = "Refreshed.".into(); }
            KeyCode::Char(c) => {
                // 단축키 검사
                self.activate_by_key(c);
            }
            _ => {}
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            // 왼쪽 사이드바 클릭 → 탭 선택
            if mouse.column < 20 && mouse.row > 0 {
                let sidebar_idx = (mouse.row as usize).saturating_sub(1);
                // Install(0), separator(1), domains(2+) 매핑
                if sidebar_idx == 0 {
                    self.selected_tab = 0;
                    self.focus_button = 0;
                } else if self.domains.is_empty() {
                    // no-op
                } else if sidebar_idx >= 2 {
                    let domain_idx = sidebar_idx - 2;
                    if domain_idx < self.domains.len() {
                        self.selected_tab = domain_idx + 1;
                        self.focus_button = 0;
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
}
