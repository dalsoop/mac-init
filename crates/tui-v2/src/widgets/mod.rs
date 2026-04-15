//! Spec 기반 공통 위젯 렌더러

use crate::spec::{Button, KvItem, Section};
use ratatui::{prelude::*, widgets::*};

pub fn render_section(frame: &mut Frame, area: Rect, section: &Section, focus_idx: usize) {
    match section {
        Section::KeyValue { title, items } => render_kv(frame, area, title, items),
        Section::Table { title, headers, rows } => render_table(frame, area, title, headers, rows),
        Section::Buttons { title, items } => render_buttons(frame, area, title, items, focus_idx),
        Section::Text { title, content } => render_text(frame, area, title, content),
    }
}

fn status_style(status: &Option<String>) -> Style {
    match status.as_deref() {
        Some("ok") => Style::default().fg(Color::Green),
        Some("error") => Style::default().fg(Color::Red),
        Some("warn") => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    }
}

fn render_kv(frame: &mut Frame, area: Rect, title: &str, items: &[KvItem]) {
    let lines: Vec<Line> = items.iter().map(|item| {
        Line::from(vec![
            Span::styled(format!("  {:<30} ", item.key), Style::default().fg(Color::Gray)),
            Span::styled(&item.value, status_style(&item.status)),
        ])
    }).collect();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", title)),
        ),
        area,
    );
}

fn render_table(frame: &mut Frame, area: Rect, title: &str, headers: &[String], rows: &[Vec<String>]) {
    let header_row = Row::new(headers.iter().map(|h| Cell::from(h.as_str())).collect::<Vec<_>>())
        .style(Style::default().fg(Color::Yellow).bold());
    let body: Vec<Row> = rows.iter().map(|r| {
        Row::new(r.iter().map(|c| Cell::from(c.as_str())).collect::<Vec<_>>())
    }).collect();
    let widths: Vec<Constraint> = headers.iter().map(|_| Constraint::Percentage(100 / headers.len().max(1) as u16)).collect();
    frame.render_widget(
        Table::new(body, widths).header(header_row).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", title)),
        ),
        area,
    );
}

fn render_buttons(frame: &mut Frame, area: Rect, title: &str, items: &[Button], focus_idx: usize) {
    let lines: Vec<Line> = items.iter().enumerate().map(|(i, b)| {
        let key = b.key.as_deref().map(|k| format!(" [{}]", k)).unwrap_or_default();
        let marker = if i == focus_idx { "▶" } else { " " };
        let style = if i == focus_idx {
            Style::default().bg(Color::DarkGray).fg(Color::White).bold()
        } else {
            Style::default().fg(Color::Cyan)
        };
        Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{} {} {}", marker, b.label, key), style),
        ])
    }).collect();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", title)),
        ),
        area,
    );
}

fn render_text(frame: &mut Frame, area: Rect, title: &str, content: &str) {
    frame.render_widget(
        Paragraph::new(content).wrap(Wrap { trim: false }).block(
            Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", title)),
        ),
        area,
    );
}

/// 섹션이 차지할 예상 높이
pub fn section_height(section: &Section) -> u16 {
    match section {
        Section::KeyValue { items, .. } => items.len() as u16 + 2,
        Section::Table { rows, .. } => rows.len() as u16 + 3,
        Section::Buttons { items, .. } => items.len() as u16 + 2,
        Section::Text { content, .. } => content.lines().count() as u16 + 2,
    }
}

