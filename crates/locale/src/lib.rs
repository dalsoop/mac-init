//! mac-locale — domains.ncl i18n 참조
//!
//! ~/.mac-app-init/locale.json (nickel export ncl/domains.ncl) 에서
//! 도메인별 label, icon, group, 버튼 라벨, 키바인딩 라벨을 읽는 공통 라이브러리.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct I18n {
    pub label: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub sections: HashMap<String, String>,
    #[serde(default)]
    pub buttons: Vec<ButtonDef>,
    #[serde(default)]
    pub keybindings: Vec<KeyBindingDef>,
    #[serde(default)]
    pub editables: Vec<EditableDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ButtonDef {
    pub command: String,
    pub label: String,
    #[serde(default)]
    pub key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeyBindingDef {
    pub key: String,
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default = "default_true")]
    pub reload: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditableDef {
    pub field: String,
    pub label: String,
    pub command: String,
    #[serde(default = "default_edit_args")]
    pub args: Vec<String>,
}

fn default_edit_args() -> Vec<String> { vec!["${value}".to_string()] }

fn default_true() -> bool { true }

#[derive(Debug, Clone, Deserialize)]
struct DomainEntry {
    i18n: I18n,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DnsPreset {
    pub label: String,
    pub primary: String,
    pub secondary: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LocaleFile {
    domains: HashMap<String, DomainEntry>,
    #[serde(default)]
    section_names: HashMap<String, String>,
    #[serde(default)]
    dns_presets: HashMap<String, DnsPreset>,
}

fn locale_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".mac-app-init/locale.json")
}

static LOCALE: OnceLock<Option<LocaleFile>> = OnceLock::new();

fn load_locale() -> &'static Option<LocaleFile> {
    LOCALE.get_or_init(|| {
        let path = locale_path();
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    })
}

/// 도메인의 i18n 정보 조회. locale.json 없으면 None.
pub fn get_i18n(domain: &str) -> Option<&'static I18n> {
    load_locale().as_ref()?.domains.get(domain).map(|d| &d.i18n)
}

/// tui-spec JSON 빌드용 — tab 객체.
pub fn tab_json(domain: &str, fallback_label: &str) -> serde_json::Value {
    match get_i18n(domain) {
        Some(i18n) => serde_json::json!({
            "label": domain,
            "label_ko": i18n.label,
            "icon": i18n.icon,
        }),
        None => serde_json::json!({
            "label": fallback_label,
        }),
    }
}

/// tui-spec JSON 빌드용 — group 문자열.
pub fn group(domain: &str, fallback: &str) -> String {
    get_i18n(domain)
        .map(|i| i.group.clone())
        .unwrap_or_else(|| fallback.to_string())
}

/// 버튼 command에 대한 한글 라벨. 없으면 fallback 반환.
pub fn button(domain: &str, command: &str, fallback: &str) -> String {
    get_i18n(domain)
        .and_then(|i| i.buttons.iter().find(|b| b.command == command))
        .map(|b| b.label.clone())
        .unwrap_or_else(|| fallback.to_string())
}

/// keybinding key에 대한 한글 라벨. 없으면 fallback 반환.
pub fn keybinding(domain: &str, key: &str, fallback: &str) -> String {
    get_i18n(domain)
        .and_then(|i| i.keybindings.iter().find(|k| k.key == key))
        .map(|k| k.label.clone())
        .unwrap_or_else(|| fallback.to_string())
}

/// 섹션 이름. 없으면 fallback 반환.
pub fn section(domain: &str, section_key: &str, fallback: &str) -> String {
    get_i18n(domain)
        .and_then(|i| i.sections.get(section_key))
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

/// tui-spec의 buttons 섹션용 JSON 배열.
/// locale.json에서 버튼 정의를 읽어 `[{ "label": "...", "command": "...", "key": "..." }, ...]` 반환.
pub fn buttons_json(domain: &str) -> Vec<serde_json::Value> {
    get_i18n(domain)
        .map(|i| {
            i.buttons.iter().map(|b| {
                let mut obj = serde_json::json!({
                    "label": b.label,
                    "command": b.command,
                });
                if !b.key.is_empty() {
                    obj.as_object_mut().unwrap().insert("key".into(), serde_json::json!(b.key));
                }
                obj
            }).collect()
        })
        .unwrap_or_default()
}

/// tui-spec의 keybindings 섹션용 JSON 배열.
pub fn keybindings_json(domain: &str) -> Vec<serde_json::Value> {
    get_i18n(domain)
        .map(|i| {
            i.keybindings.iter().map(|k| {
                serde_json::json!({
                    "key": k.key,
                    "label": k.label,
                    "command": k.command,
                    "args": k.args,
                    "confirm": k.confirm,
                    "reload": k.reload,
                })
            }).collect()
        })
        .unwrap_or_default()
}

/// 도메인의 편집 가능 필드 목록.
pub fn editables(domain: &str) -> Vec<EditableDef> {
    get_i18n(domain)
        .map(|i| i.editables.clone())
        .unwrap_or_default()
}

/// tui-spec의 editables 섹션용 JSON 배열.
pub fn editables_json(domain: &str) -> Vec<serde_json::Value> {
    get_i18n(domain)
        .map(|i| {
            i.editables.iter().map(|e| {
                serde_json::json!({
                    "field": e.field,
                    "label": e.label,
                    "command": e.command,
                    "args": e.args,
                })
            }).collect()
        })
        .unwrap_or_default()
}

/// 전체 도메인 이름 목록. locale.json에서 읽음.
pub fn get_all_domain_names() -> Vec<String> {
    load_locale().as_ref()
        .map(|l| {
            let mut names: Vec<String> = l.domains.keys().cloned().collect();
            names.sort();
            names
        })
        .unwrap_or_default()
}

/// DNS 프리셋 전체 목록. locale.json의 dns_presets에서 읽음.
pub fn dns_presets() -> HashMap<String, DnsPreset> {
    load_locale().as_ref()
        .map(|l| l.dns_presets.clone())
        .unwrap_or_default()
}

/// DNS 프리셋 이름으로 조회. 없으면 None.
pub fn dns_preset(name: &str) -> Option<DnsPreset> {
    dns_presets().get(name).cloned()
}
