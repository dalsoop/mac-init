//! Template resolution for keybinding args.

use std::collections::HashMap;

/// Resolve `${selected.<field>}` and `${toggle:<field>}` in a template string.
pub fn resolve_template(template: &str, data: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str("${"); out.push_str(after);
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
            out.push_str("${"); out.push_str(expr); out.push('}');
        }
    }
    out.push_str(rest);
    out
}
