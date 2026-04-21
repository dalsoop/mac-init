//! All render logic — pure &self reads, NO state mutation.

use super::App;
use super::types::{Focus, SidebarItem};
use crate::spec::Section;
use crate::widgets;
use ratatui::{prelude::*, widgets::*};

impl App {
    /// Main render entry point. Takes &self — no mutation allowed.
    pub fn render(&self, frame: &mut Frame) {
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
                Focus::Content => "↑↓ 항목 │ Enter 실행 │ e 수정 │ Tab 섹션 │ ←/Esc 뒤로",
            }
        };
        frame.render_widget(
            Paragraph::new(Span::styled(hints, Style::default().fg(Color::DarkGray))),
            outer[1],
        );

        // 입력 모달 오버레이
        if let Some(modal) = &self.input_modal {
            modal.render(frame);
        }
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
                    } else {
                        Style::default().fg(Color::White)
                    };
                    items.push(ListItem::new(Line::from(Span::styled(
                        "   📋 도메인 현황",
                        style,
                    ))));
                }
                SidebarItem::Domain { id, label, icon } => {
                    let selected = self.selected_tab == id.0 + 1;
                    let style = if is_cursor {
                        Style::default().bg(Color::Cyan).fg(Color::Black).bold()
                    } else if selected {
                        Style::default().fg(Color::Cyan).bold()
                    } else {
                        Style::default().fg(Color::White)
                    };
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("   {} {}", icon, label),
                        style,
                    ))));
                }
            }
        }

        if self.confirm_quit {
            items.push(ListItem::new(Line::from("")));
            items.push(ListItem::new(Line::from(Span::styled(
                " 종료? (y/n)",
                Style::default().fg(Color::Red).bold(),
            ))));
        }

        let border = if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(" mac-app-init "),
        );
        frame.render_widget(list, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        if self.selected_tab == 0 {
            self.render_install(frame, area);
            return;
        }
        let domain_idx = self.selected_tab - 1;
        if self
            .specs
            .get(domain_idx)
            .and_then(|s| s.as_ref())
            .is_none()
        {
            self.render_no_spec(frame, area);
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
        let Some(Some(spec)) = self.specs.get(domain_idx) else {
            return;
        };
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
                format!(" {}", title),
                style,
            ))));
        }

        let border = if menu_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let domain_label = spec.tab.label_ko.as_deref().unwrap_or(&spec.tab.label);
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(format!(" {} ", domain_label)),
        );
        frame.render_widget(list, area);
    }

    /// 3열: 선택된 섹션 내용만
    fn render_section_content(&self, frame: &mut Frame, area: Rect) {
        let domain_idx = self.selected_tab - 1;
        let Some(Some(spec)) = self.specs.get(domain_idx) else {
            return;
        };
        let section_idx = self
            .content_section
            .min(spec.sections.len().saturating_sub(1));
        let Some(section) = spec.sections.get(section_idx) else {
            return;
        };

        let domain = &self.domains[domain_idx];

        // output 영역 분할
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(6)])
            .split(area);

        // 섹션 렌더
        let content_focused = self.focus == Focus::Content;
        widgets::render_section(
            frame,
            chunks[0],
            section,
            self.focus_button,
            content_focused,
        );

        // output
        if !self.output.is_empty() {
            let title = if self.action_running {
                let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                static ACTION_START: std::sync::OnceLock<std::time::Instant> =
                    std::sync::OnceLock::new();
                let tick = (ACTION_START
                    .get_or_init(std::time::Instant::now)
                    .elapsed()
                    .as_millis()
                    / 66) as usize;
                format!(" {} 실행 중… ", spinners[tick % spinners.len()])
            } else {
                " Output ".to_string()
            };
            frame.render_widget(
                Paragraph::new(self.output.as_str())
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(if self.action_running {
                                Color::Yellow
                            } else {
                                Color::DarkGray
                            }))
                            .title(title),
                    ),
                chunks[1],
            );
        } else {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!("  mai run {} --help", domain),
                    Style::default().fg(Color::DarkGray),
                ))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                ),
                chunks[1],
            );
        }
    }

    fn render_install(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(8)])
            .split(area);

        if self.available.is_empty() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  `mac` 바이너리를 찾을 수 없거나 사용 가능한 도메인이 없습니다.",
                        Style::default().fg(Color::Yellow),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  터미널에서:  mac available",
                        Style::default().fg(Color::Cyan),
                    )),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Install "),
                ),
                chunks[0],
            );
        } else {
            let items: Vec<ListItem> = self
                .available
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let installed = self.is_installed(name);
                    let (marker, status, marker_style, name_style, status_style) = if installed {
                        let domain_idx = self.domains.iter().position(|d| d == name);
                        let usage = domain_idx.and_then(|idx| {
                            self.specs
                                .get(idx)
                                .and_then(|s| s.as_ref())
                                .and_then(|s| s.usage.as_ref())
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
                })
                .collect();

            // Compute install_area_top for mouse handling:
            // It's chunks[0].y + 1 (box border). We return it via last_install_area_top().
            // Since render is &self, we store it in a Cell.
            self.install_area_top.set(chunks[0].y + 1);

            frame.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(format!(
                            " 도메인 현황 — {}/{} 설치됨 ",
                            self.domains.len(),
                            self.available.len()
                        )),
                ),
                chunks[0],
            );
        }

        frame.render_widget(
            Paragraph::new(self.output.as_str())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Output — Enter/Space: 설치·삭제 토글 "),
                ),
            chunks[1],
        );
    }

    fn render_no_spec(&self, frame: &mut Frame, area: Rect) {
        let domain = self
            .domains
            .get(self.selected_tab.saturating_sub(1))
            .cloned()
            .unwrap_or_default();
        let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
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
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" {} ", domain)),
            ),
            area,
        );
    }
}
