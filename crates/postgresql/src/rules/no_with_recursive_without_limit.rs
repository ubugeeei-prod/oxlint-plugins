//! Port of `no-with-recursive-without-limit`: disallow `WITH RECURSIVE` queries
//! that have no `LIMIT` on the outer `SELECT`. Without a limit, a buggy
//! termination condition can cause the recursion to run indefinitely.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    // Only flag when the outermost SELECT carries a WITH RECURSIVE clause.
    let Some(with_clause) = field(node, "withClause") else {
        return;
    };
    let is_recursive = with_clause
        .get("recursive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !is_recursive {
        return;
    }
    // Allow if the outer SELECT already has a limitCount set.
    if field(node, "limitCount").is_some() {
        return;
    }
    ctx.report(node, "noLimit");
}
