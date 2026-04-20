//! Input modal state and handling.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use tui_input::Input;

/// Text input modal state.
pub struct InputModal {
    /// 모달 상단 라벨 (예: "Git 사용자 이름")
    pub label: String,
    /// 입력 버퍼
    pub input: Input,
    /// 확인 시 실행할 도메인
    pub domain: String,
    /// 확인 시 실행할 명령
    pub command: String,
    /// 인자 템플릿 (${value} -> 사용자 입력으로 치환)
    pub args_template: Vec<String>,
}

/// Result from handling a modal key event.
pub enum ModalAction {
    /// User pressed Enter: execute the action.
    Submit {
        value: String,
        domain: String,
        command: String,
        args: Vec<String>,
    },
    /// User pressed Esc: close modal.
    Cancel,
    /// Key was consumed by the modal (typing, backspace, etc.)
    Consumed,
}

impl InputModal {
    pub fn handle_key(&mut self, key: KeyEvent) -> ModalAction {
        match key.code {
            KeyCode::Enter => {
                let value = self.input.value().to_string();
                let args: Vec<String> = self.args_template.iter()
                    .map(|a| a.replace("${value}", &value))
                    .collect();
                ModalAction::Submit {
                    value,
                    domain: self.domain.clone(),
                    command: self.command.clone(),
                    args,
                }
            }
            KeyCode::Esc => ModalAction::Cancel,
            KeyCode::Char(c) => {
                self.input = self.input.clone().with_value(
                    format!("{}{}", self.input.value(), c)
                );
                ModalAction::Consumed
            }
            KeyCode::Backspace => {
                let v = self.input.value().to_string();
                if !v.is_empty() {
                    let new_v: String = v.chars().take(v.chars().count() - 1).collect();
                    self.input = self.input.clone().with_value(new_v);
                }
                ModalAction::Consumed
            }
            _ => ModalAction::Consumed,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
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
            .title(format!(" {} ", self.label));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        let scroll = self.input.visual_scroll(inner.width as usize);
        let input_widget = Paragraph::new(self.input.value())
            .scroll((0, scroll as u16))
            .style(Style::default().fg(Color::White));
        frame.render_widget(input_widget, inner);

        // 커서 위치
        frame.set_cursor_position((
            inner.x + (self.input.visual_cursor().saturating_sub(scroll)) as u16,
            inner.y,
        ));
    }
}
