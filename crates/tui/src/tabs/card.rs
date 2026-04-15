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
    #[serde(default)]
    mount_options: MountOptionsView,
}

#[derive(Debug, Clone, Deserialize)]
struct MountOptionsView {
    #[serde(default)]
    readonly: bool,
    #[serde(default = "default_true")]
    noappledouble: bool,
    #[serde(default = "default_true")]
    soft: bool,
    #[serde(default = "default_true")]
    nobrowse: bool,
    #[serde(default)]
    rsize: u32,
    #[serde(default)]
    wsize: u32,
}
fn default_true() -> bool { true }

impl Default for MountOptionsView {
    fn default() -> Self {
        Self { readonly: false, noappledouble: true, soft: true, nobrowse: true, rsize: 0, wsize: 0 }
    }
}

#[derive(Default)]
struct AddForm {
    name: String,
    host: String,
    user: String,
    port: String,
    scheme: String,
    password: String,
    field: usize, // 0..=5
}

const FIELD_NAMES: &[&str] = &["name", "host", "user", "port", "scheme", "password"];

enum Mode {
    Normal,
    AddForm(AddForm),
    ConfirmDelete,
}

pub struct CardTab {
    cards: Vec<Card>,
    selected: usize,
    output: String,
    show_password: bool,
    password_cache: Option<(String, String)>,
    mode: Mode,
}

impl CardTab {
    pub fn new() -> Self {
        Self {
            cards: Vec::new(),
            selected: 0,
            output: String::new(),
            show_password: false,
            password_cache: None,
            mode: Mode::Normal,
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
        // 모드별 라우팅 (모달 우선)
        match &mut self.mode {
            Mode::AddForm(_) => return self.handle_form_key(key).await,
            Mode::ConfirmDelete => return self.handle_confirm_key(key).await,
            Mode::Normal => {}
        }
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
            KeyCode::Char('a') => {
                let mut f = AddForm::default();
                f.port = "22".into();
                f.scheme = "ssh".into();
                self.mode = Mode::AddForm(f);
            }
            KeyCode::Char('d') => {
                if !self.cards.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
            }
            // 마운트 옵션 토글
            KeyCode::Char('R') => self.toggle_option("readonly").await,
            KeyCode::Char('N') => self.toggle_option("noappledouble").await,
            KeyCode::Char('S') => self.toggle_option("soft").await,
            KeyCode::Char('B') => self.toggle_option("nobrowse").await,
            _ => {}
        }
        Ok(())
    }

    async fn handle_form_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::AddForm(form) = &mut self.mode else { return Ok(()); };
        match key.code {
            KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Tab | KeyCode::Down => { form.field = (form.field + 1) % FIELD_NAMES.len(); }
            KeyCode::BackTab | KeyCode::Up => {
                form.field = (form.field + FIELD_NAMES.len() - 1) % FIELD_NAMES.len();
            }
            KeyCode::Backspace => {
                let buf = field_buf_mut(form);
                buf.pop();
            }
            KeyCode::Char(c) => {
                let buf = field_buf_mut(form);
                buf.push(c);
            }
            KeyCode::Enter => {
                if form.name.is_empty() || form.host.is_empty() || form.user.is_empty() {
                    self.output = "✗ name/host/user 필수".into();
                    return Ok(());
                }
                let port: u16 = form.port.parse().unwrap_or(22);
                let scheme = if form.scheme.is_empty() { "ssh".into() } else { form.scheme.clone() };
                let mut args: Vec<String> = vec![
                    "add".into(), form.name.clone(),
                    "--host".into(), form.host.clone(),
                    "--user".into(), form.user.clone(),
                    "--port".into(), port.to_string(),
                    "--scheme".into(), scheme,
                ];
                if !form.password.is_empty() {
                    args.push("--password".into());
                    args.push(form.password.clone());
                }
                let out = Command::new(env_binary()).args(&args).output();
                match out {
                    Ok(o) if o.status.success() => {
                        self.output = format!("✓ 추가됨: {}", form.name);
                        self.mode = Mode::Normal;
                        let _ = self.load().await;
                    }
                    Ok(o) => {
                        self.output = String::from_utf8_lossy(&o.stderr).trim().to_string();
                    }
                    Err(e) => self.output = format!("✗ {}", e),
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(c) = self.cards.get(self.selected) {
                    let name = c.name.clone();
                    let out = Command::new(env_binary()).args(["rm", &name]).output();
                    match out {
                        Ok(o) if o.status.success() => {
                            self.output = format!("✓ 삭제됨: {}", name);
                            if self.selected > 0 { self.selected -= 1; }
                            let _ = self.load().await;
                        }
                        Ok(o) => self.output = String::from_utf8_lossy(&o.stderr).trim().to_string(),
                        Err(e) => self.output = format!("✗ {}", e),
                    }
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => { self.mode = Mode::Normal; }
            _ => {}
        }
        Ok(())
    }

    async fn toggle_option(&mut self, key: &str) {
        let Some(c) = self.cards.get(self.selected) else { return; };
        let cur = match key {
            "readonly" => c.mount_options.readonly,
            "noappledouble" => c.mount_options.noappledouble,
            "soft" => c.mount_options.soft,
            "nobrowse" => c.mount_options.nobrowse,
            _ => return,
        };
        let next = if cur { "false" } else { "true" };
        let name = c.name.clone();
        let out = Command::new(env_binary())
            .args(["set-option", &name, key, next])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                self.output = format!("✓ {} {}={}", name, key, next);
                let _ = self.load().await;
            }
            Ok(o) => self.output = String::from_utf8_lossy(&o.stderr).trim().to_string(),
            Err(e) => self.output = format!("✗ {}", e),
        }
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
            Constraint::Length(11),
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
                let mo = &c.mount_options;
                let mark = |b: bool| if b { "●" } else { "○" };
                let sz = |n: u32| if n == 0 { "-".to_string() } else { format!("{}", n) };
                vec![
                    format!("name        : {}", c.name),
                    format!("scheme      : {}", c.scheme),
                    format!("user        : {}", c.user),
                    format!("host:port   : {}:{}", c.host, c.port),
                    format!("description : {}", c.description),
                    pw_line,
                    format!(
                        "options     : {} R readonly   {} N noappledouble   {} S soft   {} B nobrowse",
                        mark(mo.readonly), mark(mo.noappledouble), mark(mo.soft), mark(mo.nobrowse)
                    ),
                    format!("io chunks   : rsize={}  wsize={}", sz(mo.rsize), sz(mo.wsize)),
                ]
            }
            None => vec!["카드 없음".into()],
        };
        let detail = Paragraph::new(
            detail_lines.into_iter().map(Line::from).collect::<Vec<_>>()
        ).block(Block::bordered().title("Detail"));
        frame.render_widget(detail, layout[1]);

        // 하단 상태
        let bindings = "j/k:이동 a:add d:삭제 i:import t:test p:비번 R/N/S/B:옵션 r:새로고침";
        let footer = Paragraph::new(format!("{}  |  {}", bindings, self.output))
            .block(Block::bordered());
        frame.render_widget(footer, layout[2]);

        // 모달 (가장 위)
        match &self.mode {
            Mode::AddForm(form) => {
                let mut lines: Vec<Line> = Vec::new();
                for (i, name) in FIELD_NAMES.iter().enumerate() {
                    let buf = match i {
                        0 => &form.name, 1 => &form.host, 2 => &form.user,
                        3 => &form.port, 4 => &form.scheme, _ => &form.password,
                    };
                    let display = if i == 5 { "•".repeat(buf.chars().count()) } else { buf.clone() };
                    let marker = if i == form.field { "▶" } else { " " };
                    lines.push(Line::from(format!("{} {:<10} : {}", marker, name, display)));
                }
                lines.push(Line::from(""));
                lines.push(Line::from("Tab/↑↓: 필드 이동  Enter: 저장  Esc: 취소"));
                render_modal(frame, area, " 카드 추가 ", lines);
            }
            Mode::ConfirmDelete => {
                let name = self.cards.get(self.selected).map(|c| c.name.as_str()).unwrap_or("?");
                let lines = vec![
                    Line::from(""),
                    Line::from(format!("  '{}' 카드를 삭제하시겠습니까?", name)),
                    Line::from(""),
                    Line::from("  카드 파일 + dotenvx 비번 + Keychain 항목 모두 제거됩니다."),
                    Line::from(""),
                    Line::from("  Y: 삭제   N/Esc: 취소"),
                ];
                render_modal(frame, area, " 삭제 확인 ", lines);
            }
            Mode::Normal => {}
        }
    }
}

fn field_buf_mut(f: &mut AddForm) -> &mut String {
    match f.field {
        0 => &mut f.name,
        1 => &mut f.host,
        2 => &mut f.user,
        3 => &mut f.port,
        4 => &mut f.scheme,
        _ => &mut f.password,
    }
}

fn render_modal(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    use ratatui::widgets::Clear;
    let w = (area.width.saturating_sub(20)).min(70).max(40);
    let h = (lines.len() as u16 + 4).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let modal = Rect { x, y, width: w, height: h };
    frame.render_widget(Clear, modal);
    let p = Paragraph::new(lines)
        .block(Block::bordered().title(title).style(Style::new().fg(Color::Yellow)));
    frame.render_widget(p, modal);
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
