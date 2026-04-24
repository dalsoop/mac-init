use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::models::BrewPackage;
use crate::services;

pub struct BrewTab {
    packages: Vec<BrewPackage>,
    filtered: Vec<usize>, // indices into packages
    selected: usize,
    search: String,
    searching: bool,
    scroll_offset: usize,
    output: String,
}

impl BrewTab {
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            search: String::new(),
            searching: false,
            scroll_offset: 0,
            output: String::new(),
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.packages = services::brew::list_installed()?;
        self.apply_filter();
        Ok(())
    }

    fn apply_filter(&mut self) {
        let query = self.search.to_lowercase();
        self.filtered = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                query.is_empty()
                    || p.name.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn selected_package(&self) -> Option<&BrewPackage> {
        self.filtered
            .get(self.selected)
            .and_then(|&i| self.packages.get(i))
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // search
                Constraint::Min(0),   // table
            ])
            .split(chunks[0]);

        // Search
        let search_text = if self.searching {
            format!("/{}", self.search)
        } else if self.search.is_empty() {
            format!(" {} packages", self.filtered.len())
        } else {
            format!("/{} ({} results)", self.search, self.filtered.len())
        };
        let search = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if self.searching {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                })
                .title(" Search "),
        );
        frame.render_widget(search, left[0]);

        // Table
        let header = Row::new(vec!["Type", "Name", "Version"])
            .style(Style::default().fg(Color::Yellow).bold())
            .bottom_margin(0);

        let visible_height = left[1].height.saturating_sub(2) as usize;
        // Adjust scroll
        let scroll = if self.selected >= self.scroll_offset + visible_height {
            self.selected.saturating_sub(visible_height - 1)
        } else if self.selected < self.scroll_offset {
            self.selected
        } else {
            self.scroll_offset
        };

        let rows: Vec<Row> = self
            .filtered
            .iter()
            .skip(scroll)
            .take(visible_height)
            .enumerate()
            .map(|(vis_i, &pkg_i)| {
                let pkg = &self.packages[pkg_i];
                let type_tag = if pkg.is_cask { "cask" } else { "formula" };
                let style = if scroll + vis_i == self.selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else if pkg.outdated {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };
                Row::new(vec![type_tag.to_string(), pkg.name.clone(), pkg.version.clone()])
                    .style(style)
            })
            .collect();

        let table = Table::new(rows, [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Min(10),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Packages "),
        );
        frame.render_widget(table, left[1]);

        // Right: Details + Output
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);

        // Details
        let detail_text = if let Some(pkg) = self.selected_package() {
            let status = if pkg.outdated {
                "Update available"
            } else {
                "Installed"
            };
            let type_str = if pkg.is_cask { "Cask" } else { "Formula" };
            format!(
                "Name: {}\nType: {}\nVersion: {}\nStatus: {}",
                pkg.name,
                type_str,
                pkg.version,
                status,
            )
        } else {
            "No package selected".to_string()
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

        // Output
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
                KeyCode::Esc => {
                    self.searching = false;
                }
                KeyCode::Enter => {
                    self.searching = false;
                }
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
            KeyCode::Char('/') => {
                self.searching = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Char('u') => {
                if let Some(pkg) = self.selected_package().cloned() {
                    if pkg.outdated {
                        self.output = services::brew::upgrade(&pkg.name, pkg.is_cask)?;
                        self.load().await?;
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(pkg) = self.selected_package().cloned() {
                    self.output = services::brew::uninstall(&pkg.name, pkg.is_cask)?;
                    self.load().await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
