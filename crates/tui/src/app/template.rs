//! Template resolution for keybinding args.

use crate::spec::DomainSpec;
use crate::spec::Section;
use std::collections::HashMap;

/// Resolve `${selected.<field>}` and `${toggle:<field>}` in a template string.
pub fn resolve_template(template: &str, data: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str("${");
            out.push_str(after);
            rest = "";
            break;
        };
        let expr = &after[..end];
        rest = &after[end + 1..];
        if let Some(field) = expr.strip_prefix("selected.") {
            out.push_str(data.get(field).map(|s| s.as_str()).unwrap_or(""));
        } else if let Some(field) = expr.strip_prefix("toggle:") {
            let cur = data.get(field).map(|s| s.as_str()).unwrap_or("false");
            out.push_str(if cur == "true" { "false" } else { "true" });
        } else {
            out.push_str("${");
            out.push_str(expr);
            out.push('}');
        }
    }
    out.push_str(rest);
    out
}

/// Get the data map for the currently selected item.
///
/// Looks at the current section first, then falls back to `list_section`.
pub fn selected_item_data(
    spec: &DomainSpec,
    content_section: usize,
    focus_button: usize,
) -> HashMap<String, String> {
    let empty = HashMap::new();

    // 1) 현재 포커스된 섹션에서 KV 데이터 시도
    let section_idx = content_section.min(spec.sections.len().saturating_sub(1));
    if let Some(Section::KeyValue { items, .. }) = spec.sections.get(section_idx) {
        if !items.is_empty() {
            let idx = focus_button.min(items.len() - 1);
            let item = &items[idx];
            let mut data = item.data.clone();
            data.entry("key".into()).or_insert(item.key.clone());
            data.entry("value".into()).or_insert(item.value.clone());
            data.entry("name".into()).or_insert(item.key.clone());
            return data;
        }
    }

    // 2) fallback: list_section 지정된 섹션에서
    if let Some(list_title) = spec.list_section.as_ref() {
        for section in &spec.sections {
            if let Section::KeyValue { title, items } = section {
                if title != list_title {
                    continue;
                }
                if items.is_empty() {
                    return empty;
                }
                let idx = focus_button.min(items.len() - 1);
                let item = &items[idx];
                let mut data = item.data.clone();
                data.entry("key".into()).or_insert(item.key.clone());
                data.entry("value".into()).or_insert(item.value.clone());
                data.entry("name".into()).or_insert(item.key.clone());
                return data;
            }
        }
    }
    empty
}
