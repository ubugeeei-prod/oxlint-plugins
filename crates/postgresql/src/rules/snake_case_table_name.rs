//! Port of `snake-case-table-name`: table names must be snake_case.
//! Unquoted identifiers are folded to lowercase by PostgreSQL; quoted
//! identifiers preserve case and can introduce inconsistencies.

use serde_json::Value;

use crate::ast::is_type;
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

fn is_snake_case(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let relname = match node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
    {
        Some(n) => n,
        None => return,
    };

    if is_snake_case(relname) {
        return;
    }

    // Check the allow list from options
    let allowed = ctx
        .options
        .get(0)
        .and_then(|o| o.get("allow"))
        .and_then(|v| v.as_array())
        .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some(relname)));
    if allowed {
        return;
    }

    ctx.report_data(
        node,
        "notSnakeCase",
        SmallVec::from_iter([DiagnosticDatum {
            key: CompactString::from("name"),
            value: CompactString::from(relname),
        }]),
    );
}
