//! Sidebar building and navigation.

use super::types::{DomainId, SidebarItem};
use crate::spec::DomainSpec;

/// 사이드바 그룹 정의. 순서 = 화면 표시 순서.
const GROUPS: &[(&str, &str)] = &[
    ("init", "인입"),
    ("infra", "인프라"),
    ("auto", "자동화"),
    ("dev", "개발"),
    ("finder", "Finder"),
    ("system", "시스템"),
    ("other", "기타"),
];

/// 도메인 -> 그룹 매핑. spec.group 이 없으면 여기서 fallback.
fn default_group(domain: &str) -> &'static str {
    match domain {
        "mount" | "env" | "host" | "network" | "ssh" | "proxmox" | "synology" => "infra",
        "cron" | "files" | "sd-backup" => "auto",
        "git" | "vscode" | "container" => "dev",
        "quickaction" => "finder",
        "keyboard" | "shell" | "bootstrap" | "wireguard" | "tmux" => "system",
        _ => "other",
    }
}

/// Build the sidebar item list from domains and their specs.
pub fn build_sidebar(domains: &[String], specs: &[Option<DomainSpec>]) -> Vec<SidebarItem> {
    let mut items = Vec::new();
    for &(group_id, group_label) in GROUPS {
        let is_init = group_id == "init";
        let mut has_domains = false;
        let mut group_domains = Vec::new();
        for (i, domain) in domains.iter().enumerate() {
            let spec_group = specs[i]
                .as_ref()
                .and_then(|s| s.group.as_deref())
                .unwrap_or_else(|| default_group(domain));
            if spec_group != group_id {
                continue;
            }
            let label = specs[i]
                .as_ref()
                .map(|s| {
                    s.tab
                        .label_ko
                        .as_deref()
                        .unwrap_or(&s.tab.label)
                        .to_string()
                })
                .unwrap_or_else(|| domain.clone());
            let icon = specs[i]
                .as_ref()
                .and_then(|s| s.tab.icon.clone())
                .unwrap_or_default();
            group_domains.push((DomainId(i), label, icon));
            has_domains = true;
        }
        if !is_init && !has_domains {
            continue;
        }
        items.push(SidebarItem::GroupHeader(group_label.to_string()));
        if is_init {
            items.push(SidebarItem::Install);
        }
        for (id, label, icon) in group_domains {
            items.push(SidebarItem::Domain { id, label, icon });
        }
    }
    items
}

/// Move sidebar cursor, skipping group headers. Returns new cursor position.
pub fn sidebar_move(items: &[SidebarItem], current: usize, dir: i32) -> usize {
    let len = items.len();
    if len == 0 {
        return current;
    }
    let has_selectable = items
        .iter()
        .any(|i| !matches!(i, SidebarItem::GroupHeader(_)));
    if !has_selectable {
        return current;
    }
    let mut next = current as i32 + dir;
    for _ in 0..len {
        if next < 0 {
            next = len as i32 - 1;
        }
        if next >= len as i32 {
            next = 0;
        }
        if !matches!(items[next as usize], SidebarItem::GroupHeader(_)) {
            break;
        }
        next += dir;
    }
    next.clamp(0, len as i32 - 1) as usize
}
