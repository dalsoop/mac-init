use ratatui::{prelude::*, widgets::Tabs};

use crate::tabs::TabId;

pub fn render_tabbar(frame: &mut Frame, area: Rect, active: &TabId) {
    let all = TabId::all();
    let titles: Vec<String> = all
        .iter()
        .enumerate()
        .map(|(i, t)| format!(" {}:{} ", i + 1, t.label()))
        .collect();

    let tabs = Tabs::new(titles)
        .select(active.index())
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .bold()
                .bg(Color::DarkGray),
        )
        .divider("|");

    frame.render_widget(tabs, area);
}
