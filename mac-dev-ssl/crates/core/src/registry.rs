//! 도메인 레지스트리 로더.
//! `nickel` CLI가 PATH에 있으면 `ncl/domains.ncl`을 JSON export 해서 사용.
//! 없으면 컴파일 타임에 내장된 폴백 문자열 사용.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    pub domains: BTreeMap<String, Domain>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Domain {
    pub name: String,
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Tags,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Tags {
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub layer: Option<String>,
}

fn default_true() -> bool {
    true
}

// 컴파일 타임에 포함된 ncl 원본 (폴백 / 참고용)
const DOMAINS_NCL: &str = include_str!("../../../ncl/domains.ncl");
const CONTRACT_NCL: &str = include_str!("../../../ncl/contract.ncl");

impl Registry {
    pub fn load() -> anyhow::Result<Self> {
        if crate::common::has_cmd("nickel") {
            match Self::load_via_nickel() {
                Ok(reg) => return Ok(reg),
                Err(e) => {
                    eprintln!("[mac-dev-ssl] ⚠ Nickel export 실패 — 폴백 사용: {e}");
                    eprintln!("  (ncl/domains.ncl 점검: `nickel eval ncl/domains.ncl`)");
                }
            }
        }
        Self::fallback()
    }

    fn load_via_nickel() -> anyhow::Result<Self> {
        // contract.ncl import를 위해 두 파일을 같은 tmp 디렉토리에 배치
        let tmp_dir = std::env::temp_dir().join("mac-dev-ssl-ncl");
        std::fs::create_dir_all(&tmp_dir)?;
        let domains_path = tmp_dir.join("domains.ncl");
        let contract_path = tmp_dir.join("contract.ncl");
        std::fs::write(&contract_path, CONTRACT_NCL)?;
        std::fs::write(&domains_path, DOMAINS_NCL)?;
        let json = crate::common::run_capture(
            "nickel",
            &["export", "--format", "json", domains_path.to_str().unwrap()],
        )?;
        Ok(serde_json::from_str(&json)?)
    }

    fn fallback() -> anyhow::Result<Self> {
        // 최소한의 하드코드 폴백 — nickel 없어도 빌드는 되어야 함.
        // 실제 값은 CI/빌드 단계에서 nickel eval로 검증되므로 여기선 스텁만.
        Ok(Registry {
            domains: BTreeMap::new(),
        })
    }

    pub fn names(&self) -> Vec<&str> {
        self.domains.keys().map(|s| s.as_str()).collect()
    }
}
