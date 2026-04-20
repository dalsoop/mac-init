//! TUI spec 빌더.
//! 14개 도메인의 print_tui_spec() 공통 구조를 추상화.
//!
//! ```rust
//! use mac_common::tui_spec::TuiSpec;
//!
//! let spec = TuiSpec::new("env")
//!     .refresh(30)
//!     .usage(true, "카드 3개")
//!     .list_section("상태")
//!     .kv("상태", items)
//!     .buttons()          // locale.json에서 자동 로드
//!     .text("안내", "도움말 텍스트")
//!     .build();
//! println!("{}", spec);
//! ```

use serde_json::Value;

pub struct TuiSpec {
    domain: String,
    tab: Value,
    group: String,
    refresh_interval: u32,
    usage: Option<Value>,
    list_section: Option<String>,
    sections: Vec<Value>,
    keybindings: Vec<Value>,
    editables: Vec<Value>,
}

impl TuiSpec {
    /// 도메인 이름으로 생성. tab/group/keybindings는 locale.json에서 자동 로드.
    pub fn new(domain: &str) -> Self {
        Self {
            domain: domain.to_string(),
            tab: mac_locale::tab_json(domain, domain),
            group: mac_locale::group(domain, "other"),
            refresh_interval: 0,
            usage: None,
            list_section: None,
            sections: Vec::new(),
            keybindings: mac_locale::keybindings_json(domain),
            editables: mac_locale::editables_json(domain),
        }
    }

    pub fn refresh(mut self, secs: u32) -> Self {
        self.refresh_interval = secs;
        self
    }

    pub fn usage(mut self, active: bool, summary: &str) -> Self {
        self.usage = Some(serde_json::json!({
            "active": active,
            "summary": summary,
        }));
        self
    }

    pub fn list_section(mut self, title: &str) -> Self {
        self.list_section = Some(title.to_string());
        self
    }

    /// key-value 섹션 추가.
    pub fn kv(mut self, title: &str, items: Vec<Value>) -> Self {
        self.sections.push(serde_json::json!({
            "kind": "key-value",
            "title": title,
            "items": items,
        }));
        self
    }

    /// table 섹션 추가.
    pub fn table(mut self, title: &str, headers: Vec<&str>, rows: Vec<Value>) -> Self {
        self.sections.push(serde_json::json!({
            "kind": "table",
            "title": title,
            "headers": headers,
            "rows": rows,
        }));
        self
    }

    /// locale.json에서 버튼을 읽어서 "실행" 섹션 추가.
    pub fn buttons(mut self) -> Self {
        let items = mac_locale::buttons_json(&self.domain);
        self.sections.push(serde_json::json!({
            "kind": "buttons",
            "title": "실행",
            "items": items,
        }));
        self
    }

    /// 커스텀 버튼 배열로 버튼 섹션 추가 (동적 토글 등).
    pub fn buttons_custom(mut self, title: &str, items: Vec<Value>) -> Self {
        self.sections.push(serde_json::json!({
            "kind": "buttons",
            "title": title,
            "items": items,
        }));
        self
    }

    /// text 섹션 추가.
    pub fn text(mut self, title: &str, content: &str) -> Self {
        self.sections.push(serde_json::json!({
            "kind": "text",
            "title": title,
            "content": content,
        }));
        self
    }

    /// 커스텀 keybindings 추가 (locale 외 동적 키바인딩).
    pub fn extra_keybinding(mut self, kb: Value) -> Self {
        self.keybindings.push(kb);
        self
    }

    /// JSON 문자열로 빌드 + 출력.
    /// sections가 비어있으면 panic (개발 시점에 잡히도록).
    pub fn print(self) {
        assert!(!self.sections.is_empty(),
            "[{}] TuiSpec: sections가 비어있음 — .kv() 또는 .buttons() 필요", self.domain);
        println!("{}", self.build());
    }

    /// JSON 문자열로 빌드.
    pub fn build(self) -> String {
        let mut spec = serde_json::json!({
            "tab": self.tab,
            "group": self.group,
            "refresh_interval": self.refresh_interval,
            "sections": self.sections,
            "keybindings": self.keybindings,
        });

        let obj = spec.as_object_mut().unwrap();
        if let Some(usage) = self.usage {
            obj.insert("usage".into(), usage);
        }
        if let Some(ls) = self.list_section {
            obj.insert("list_section".into(), Value::String(ls));
        }
        if !self.editables.is_empty() {
            obj.insert("editables".into(), Value::Array(self.editables));
        }

        serde_json::to_string_pretty(&spec).unwrap_or_default()
    }
}

// ── KV 아이템 헬퍼 ──

/// key-value 아이템 생성.
pub fn kv_item(key: &str, value: &str, status: &str) -> Value {
    serde_json::json!({
        "key": key,
        "value": value,
        "status": status,
    })
}

/// key-value 아이템 + data 맵.
pub fn kv_item_data(key: &str, value: &str, status: &str, data: Value) -> Value {
    serde_json::json!({
        "key": key,
        "value": value,
        "status": status,
        "data": data,
    })
}
