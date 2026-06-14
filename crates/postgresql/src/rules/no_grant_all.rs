//! Port of `no-grant-all`: disallow `GRANT ALL` / `GRANT ALL PRIVILEGES`.
//!
//! libpg_query represents `GRANT ALL` as a `GrantStmt` with no `privileges`
//! array (or an empty one). A specific privilege list populates the array.
//! `REVOKE ALL` is the safe direction (taking everything back), so only
//! `GrantStmt` with `is_grant: true` is flagged.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "GrantStmt") {
        return;
    }
    let is_grant = node
        .get("is_grant")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !is_grant {
        return;
    }
    // `GRANT ALL` — privileges array absent or empty.
    if array_field(node, "privileges").is_some_and(|p| !p.is_empty()) {
        return;
    }
    ctx.report(node, "noGrantAll");
}
