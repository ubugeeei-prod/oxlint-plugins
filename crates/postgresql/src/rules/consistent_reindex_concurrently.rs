//! Port of `consistent-reindex-concurrently`: enforce a consistent stance on
//! `CONCURRENTLY` for `REINDEX` (either always require it, or always forbid it).
//!
//! Default style `"always"` requires `REINDEX ... CONCURRENTLY ...`.
//! Style `"never"` forbids the `CONCURRENTLY` option.
//!
//! Reports on the `ReindexStmt` node itself.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

/// Returns true when the `params` array of a `ReindexStmt` contains a
/// `DefElem` with `defname === "concurrently"`.
fn is_concurrent(node: &Value) -> bool {
    let params = match node.get("params").and_then(Value::as_array) {
        Some(p) => p,
        None => return false,
    };
    params.iter().any(|p| {
        is_type(p, "DefElem") && p.get("defname").and_then(Value::as_str) == Some("concurrently")
    })
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ReindexStmt") {
        return;
    }

    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");

    let concurrent = is_concurrent(node);

    if style == "always" && !concurrent {
        ctx.report(node, "preferReindexConcurrently");
    } else if style == "never" && concurrent {
        ctx.report(node, "unexpectedReindexConcurrently");
    }
}
