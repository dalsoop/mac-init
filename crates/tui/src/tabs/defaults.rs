use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use mac_host_core::models::defaults::DefaultEntry;
use mac_host_core::defaults;

enum View {
    Domains,
    Entries,
}

pub struct DefaultsTab {
    domains: Vec<String>,
    filtered_domains: Vec<usize>,
    entries: Vec<DefaultEntry>,
    selected: usize,
    search: String,
    searching: bool,
    view: View,
    current_domain: String,
    output: String,
}

impl DefaultsTab {
    pub fn new() -> Self {
        Self {
            domains: Vec::new(),
            filtered_domains: Vec::new(),
            entries: Vec::new(),
            selected: 0,
            search: String::new(),
            searching: false,
            view: View::Domains,
            current_domain: String::new(),
            output: String::new(),
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.domains = defaults::list_domains();
        self.apply_filter();
        Ok(())
    }

    fn apply_filter(&mut self) {
        let query = self.search.to_lowercase();
        match self.view {
            View::Domains => {
                self.filtered_domains = self
                    .domains
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| query.is_empty() || d.to_lowercase().contains(&query))
                    .map(|(i, _)| i)
                    .collect();
            }
            View::Entries => {
                // entries are shown directly, filter not applied to indices
            }
        }
        if self.selected >= self.item_count() {
            self.selected = self.item_count().saturating_sub(1);
        }
    }

    fn item_count(&self) -> usize {
        match self.view {
            View::Domains => self.filtered_domains.len(),
            View::Entries => self.entries.len(),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(chunks[0]);

        // Search
        let title = match self.view {
            View::Domains => format!(" Domains ({}) ", self.filtered_domains.len()),
            View::Entries => format!(" {} ({}) ", self.current_domain, self.entries.len()),
        };
        let search_text = if self.searching {
            format!("/{}", self.search)
        } else if self.search.is_empty() {
            " type / to search".to_string()
        } else {
            format!("/{}", self.search)
        };
        let search = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if self.searching {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                })
                .title(title),
        );
        frame.render_widget(search, left[0]);

        // List
        let visible_height = left[1].height.saturating_sub(2) as usize;
        let scroll = self.selected.saturating_sub(visible_height.saturating_sub(1));

        match self.view {
            View::Domains => {
                let items: Vec<ListItem> = self
                    .filtered_domains
                    .iter()
                    .skip(scroll)
                    .take(visible_height)
                    .enumerate()
                    .map(|(vis_i, &idx)| {
                        let style = if scroll + vis_i == self.selected {
                            Style::default().bg(Color::DarkGray).fg(Color::White)
                        } else if self.domains[idx].starts_with("com.apple.") {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::Gray)
                        };
                        ListItem::new(self.domains[idx].as_str()).style(style)
                    })
                    .collect();

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Domains "),
                );
                frame.render_widget(list, left[1]);
            }
            View::Entries => {
                let rows: Vec<Row> = self
                    .entries
                    .iter()
                    .skip(scroll)
                    .take(visible_height)
                    .enumerate()
                    .map(|(vis_i, entry)| {
                        let style = if scroll + vis_i == self.selected {
                            Style::default().bg(Color::DarkGray).fg(Color::White)
                        } else {
                            Style::default()
                        };
                        Row::new(vec![
                            entry.key.clone(),
                            entry.value.clone(),
                            entry.value_type.clone(),
                        ])
                        .style(style)
                    })
                    .collect();

                let header = Row::new(vec!["Key", "Value", "Type"])
                    .style(Style::default().fg(Color::Yellow).bold());

                let table = Table::new(rows, [
                    Constraint::Percentage(40),
                    Constraint::Percentage(40),
                    Constraint::Percentage(20),
                ])
                .header(header)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(format!(" {} ", self.current_domain)),
                );
                frame.render_widget(table, left[1]);
            }
        }

        // Right: detail/output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let detail_text = match self.view {
            View::Domains => {
                if let Some(&idx) = self.filtered_domains.get(self.selected) {
                    format!("Domain: {}\n\nPress Enter to view settings", self.domains[idx])
                } else {
                    "No domain selected".to_string()
                }
            }
            View::Entries => {
                if let Some(entry) = self.entries.get(self.selected) {
                    format!(
                        "Domain: {}\nKey: {}\nValue: {}\nType: {}",
                        entry.domain, entry.key, entry.value, entry.value_type
                    )
                } else {
                    "No entry selected".to_string()
                }
            }
        };
        let details = Paragraph::new(detail_text)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Details "),
            );
        frame.render_widget(details, right[0]);

        let output = Paragraph::new(self.output.as_str())
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Output "),
            );
        frame.render_widget(output, right[1]);
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.searching {
            match key.code {
                KeyCode::Esc => self.searching = false,
                KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.search.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.search.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.item_count() {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => match self.view {
                View::Domains => {
                    if let Some(&idx) = self.filtered_domains.get(self.selected) {
                        let domain = self.domains[idx].clone();
                        self.entries = defaults::read_domain(&domain);
                        self.current_domain = domain;
                        self.view = View::Entries;
                        self.selected = 0;
                        self.search.clear();
                    }
                }
                View::Entries => {}
            },
            KeyCode::Esc | KeyCode::Backspace => {
                if matches!(self.view, View::Entries) {
                    self.view = View::Domains;
                    self.entries.clear();
                    self.selected = 0;
                    self.search.clear();
                    self.apply_filter();
                }
            }
            _ => {}
        }
        Ok(())
    }
}
