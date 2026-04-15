//! Domain UI Specification
//!
//! 도메인 바이너리가 `tui-spec` 명령으로 반환하는 JSON 구조.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSpec {
    pub tab: TabInfo,
    #[serde(default)]
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Section {
    KeyValue {
        title: String,
        items: Vec<KvItem>,
    },
    Table {
        title: String,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Buttons {
        title: String,
        items: Vec<Button>,
    },
    Text {
        title: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvItem {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub status: Option<String>, // "ok" | "error" | "warn" | null
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub key: Option<String>,
}
