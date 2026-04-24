use ratatui::{prelude::*, widgets::Tabs};

use crate::tabs::TabId;

pub fn render_tabbar(frame: &mut Frame, area: Rect, active: &TabId) {
    let titles: Vec<String> = TabId::ALL
        .iter()
        .map(|t| format!(" {}:{} ", t.key(), t.label()))
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
