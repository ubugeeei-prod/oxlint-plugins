//! Port of `snake-case-column-name`: require column names to be snake_case.
//! PostgreSQL preserves the case of quoted identifiers, so a mixed-case quoted
//! name forces every consumer to quote-match it.

use serde_json::Value;

use crate::ast::is_type;
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Mirrors upstream's `/^[a-z][a-z0-9_]*$/`.
fn is_snake_case(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    // Upstream: `const name = node.colname; if (typeof name !== "string") return`.
    let Some(name) = node.get("colname").and_then(Value::as_str) else {
        return;
    };
    // `allow` option set membership (default `[]`).
    let allowed = ctx
        .options
        .get(0)
        .and_then(|o| o.get("allow"))
        .and_then(Value::as_array)
        .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some(name)));
    if allowed {
        return;
    }
    if is_snake_case(name) {
        return;
    }
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("name"),
        value: CompactString::from(name),
    });
    ctx.report_data(node, "notSnakeCase", data);
}
