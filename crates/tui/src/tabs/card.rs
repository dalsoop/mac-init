//! Card 탭 — env 도메인의 카드 목록/관리 UI.
//!
//! mac-domain-env CLI 를 통해 읽기/쓰기. 탭 자체는 얇은 뷰.
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
struct Card {
    name: String,
    host: String,
    user: String,
    port: u16,
    scheme: String,
    #[serde(default)]
    description: String,
}

pub struct CardTab {
    cards: Vec<Card>,
    selected: usize,
    output: String,
    show_password: bool,
    password_cache: Option<(String, String)>, // (name, password)
}

impl CardTab {
    pub fn new() -> Self {
        Self {
            cards: Vec::new(),
            selected: 0,
            output: String::new(),
            show_password: false,
            password_cache: None,
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        self.cards.clear();
        self.password_cache = None;

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let dir = PathBuf::from(&home).join(".mac-app-init/cards");
        if !dir.exists() {
            self.output = format!("카드 없음. `mac run env import` 로 이관하세요.");
            return Ok(());
        }
        let it = match std::fs::read_dir(&dir) {
            Ok(x) => x,
            Err(e) => { self.output = format!("cards 디렉터리 읽기 실패: {}", e); return Ok(()); }
        };
        for e in it.filter_map(|x| x.ok()) {
            if e.path().extension().and_then(|s| s.to_str()) != Some("json") { continue; }
            let Ok(content) = std::fs::read_to_string(e.path()) else { continue; };
            if let Ok(c) = serde_json::from_str::<Card>(&content) {
                self.cards.push(c);
            }
        }
        self.cards.sort_by(|a, b| a.name.cmp(&b.name));
        if self.selected >= self.cards.len() { self.selected = 0; }
        Ok(())
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.cards.is_empty() && self.selected + 1 < self.cards.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 { self.selected -= 1; }
            }
            KeyCode::Char('r') => { self.load().await?; self.output = "reload".into(); }
            KeyCode::Char('i') => { self.run_import().await; self.load().await?; }
            KeyCode::Char('t') => { self.run_test().await; }
            KeyCode::Char('p') => {
                self.show_password = !self.show_password;
                if self.show_password { self.fetch_password().await; }
            }
            _ => {}
        }
        Ok(())
    }

    async fn run_import(&mut self) {
        let out = Command::new(env_binary()).args(["import"]).output();
        match out {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                self.output = s.lines().last().unwrap_or("import 완료").to_string();
            }
            Err(e) => self.output = format!("✗ import 실패: {}", e),
        }
    }

    async fn run_test(&mut self) {
        let Some(c) = self.cards.get(self.selected) else { return; };
        let out = Command::new(env_binary()).args(["test", &c.name]).output();
        match out {
            Ok(o) => {
                let txt = if o.status.success() { &o.stdout } else { &o.stderr };
                self.output = String::from_utf8_lossy(txt).trim().to_string();
            }
            Err(e) => self.output = format!("✗ test 실패: {}", e),
        }
    }

    async fn fetch_password(&mut self) {
        let Some(c) = self.cards.get(self.selected) else { return; };
        let out = Command::new(env_binary()).args(["get-password", &c.name]).output();
        if let Ok(o) = out {
            if o.status.success() {
                let pw = String::from_utf8_lossy(&o.stdout).to_string();
                self.password_cache = Some((c.name.clone(), pw));
                return;
            }
        }
        self.password_cache = Some((c.name.clone(), "(없음)".into()));
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(10),
            Constraint::Length(3),
        ]).split(area);

        // 목록
        let rows: Vec<Row> = self.cards.iter().enumerate().map(|(i, c)| {
            let style = if i == self.selected {
                Style::new().bg(Color::DarkGray).fg(Color::Yellow)
            } else {
                Style::new()
            };
            Row::new(vec![
                c.name.clone(),
                c.scheme.clone(),
                c.user.clone(),
                format!("{}:{}", c.host, c.port),
                c.description.clone(),
            ]).style(style)
        }).collect();
        let widths = [
            Constraint::Length(14),
            Constraint::Length(7),
            Constraint::Length(14),
            Constraint::Length(22),
            Constraint::Min(10),
        ];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["NAME", "SCHEME", "USER", "HOST:PORT", "DESCRIPTION"])
                    .style(Style::new().bold())
            )
            .block(Block::bordered().title(format!("Cards ({})", self.cards.len())));
        frame.render_widget(table, layout[0]);

        // 상세
        let detail_lines = match self.cards.get(self.selected) {
            Some(c) => {
                let pw_line = if self.show_password {
                    match &self.password_cache {
                        Some((n, pw)) if n == &c.name => format!("password    : {}", pw),
                        _ => "password    : (로딩 중... p 다시)".into(),
                    }
                } else {
                    "password    : (p 로 표시)".into()
                };
                vec![
                    format!("name        : {}", c.name),
                    format!("scheme      : {}", c.scheme),
                    format!("user        : {}", c.user),
                    format!("host:port   : {}:{}", c.host, c.port),
                    format!("description : {}", c.description),
                    pw_line,
                ]
            }
            None => vec!["카드 없음".into()],
        };
        let detail = Paragraph::new(
            detail_lines.into_iter().map(Line::from).collect::<Vec<_>>()
        ).block(Block::bordered().title("Detail"));
        frame.render_widget(detail, layout[1]);

        // 하단 상태
        let bindings = "j/k: 이동  i: import  t: test  p: 비번 토글  r: 새로고침";
        let footer = Paragraph::new(format!("{}  |  {}", bindings, self.output))
            .block(Block::bordered());
        frame.render_widget(footer, layout[2]);
    }
}

fn env_binary() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let candidates = [
        PathBuf::from(&home).join(".mac-app-init/domains/mac-domain-env"),
        PathBuf::from("./target/debug/mac-domain-env"),
        PathBuf::from("./target/release/mac-domain-env"),
    ];
    for c in &candidates {
        if c.exists() { return c.clone(); }
    }
    PathBuf::from("mac-domain-env")
}
