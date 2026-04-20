//! Domain UI Specification
//!
//! 도메인 바이너리가 `tui-spec` 명령으로 반환하는 JSON 구조.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSpec {
    pub tab: TabInfo,
    /// 도메인이 속하는 번들/그룹 (사이드바 2단 트리용).
    /// 없으면 "기타" 로 분류.
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub sections: Vec<Section>,
    /// 전역 키 바인딩 — TUI 가 받아서 도메인 CLI 실행.
    #[serde(default)]
    pub keybindings: Vec<KeyBinding>,
    /// 선택 대상이 되는 섹션 제목 (예: "Cards"). KeyValue / Table 섹션 이름.
    /// 지정되면 TUI 가 해당 섹션에서 항목 포커스를 관리하고
    /// ${selected.*} 치환에 사용.
    #[serde(default)]
    pub list_section: Option<String>,
    /// 자동 갱신 간격 (초). 0 이면 자동 갱신 안 함.
    /// SD 백업 같이 상태가 실시간으로 바뀌는 도메인에서 사용.
    #[serde(default)]
    pub refresh_interval: u32,
    /// 도메인 사용 여부. true = "✓ 사용", false = "○ 미사용".
    /// 각 도메인이 자체 기준으로 판별 (카드 1+, 마운트 1+, job 1+ 등).
    #[serde(default)]
    pub usage: Option<UsageInfo>,
    /// 편집 가능 필드 정의 (locale.json → tui-spec으로 전달).
    #[serde(default)]
    pub editables: Vec<EditableField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableField {
    /// KV 상태 항목의 key (매칭용)
    pub field: String,
    /// 입력 모달에 표시할 라벨
    pub label: String,
    /// 도메인 CLI 서브커맨드
    pub command: String,
    /// 명령 인자. ${value}가 사용자 입력으로 치환됨
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    /// 사용 중이면 true.
    pub active: bool,
    /// 한 줄 요약 (예: "카드 3개", "비활성", "Docker 실행 중").
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    /// 단일 문자 ("R"), 대소문자 구분.
    pub key: String,
    /// 상태바에 표시될 설명.
    pub label: String,
    /// 도메인 바이너리에 전달할 서브커맨드 (예: "set-option").
    pub command: String,
    /// 인자. 템플릿 변수 지원:
    ///   ${selected.name}     — list_section 에서 선택된 항목의 name
    ///   ${selected.<field>}  — 그 항목의 임의 필드 (data 메타에서)
    ///   ${toggle:<key>}      — 선택 항목의 <key> 값 반대로 (true↔false)
    ///   ${prompt:<label>}    — 모달 입력 (1단계에선 미구현, 플레이스홀더로 둠)
    #[serde(default)]
    pub args: Vec<String>,
    /// true 면 실행 전 y/n 확인 모달.
    #[serde(default)]
    pub confirm: bool,
    /// 실행 후 spec reload (상태 갱신). 기본 true.
    #[serde(default = "default_true")]
    pub reload: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub label: String,
    /// 한국어 라벨. 없으면 label 그대로.
    #[serde(default)]
    pub label_ko: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    /// 도메인 한 줄 설명.
    #[serde(default)]
    pub description: Option<String>,
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
    /// 템플릿 치환용 추가 메타. ${selected.<name>} 로 접근.
    /// 예) { "name": "synology", "readonly": "false" }
    #[serde(default)]
    pub data: std::collections::HashMap<String, String>,
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
