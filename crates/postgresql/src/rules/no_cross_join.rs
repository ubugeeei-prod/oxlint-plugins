//! Port of `no-cross-join`: disallow `CROSS JOIN` (unqualified cartesian
//! product) — a `JoinExpr` with `jointype == "JOIN_INNER"` and no `quals`,
//! `usingClause`, or `isNatural` flag.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "JoinExpr") {
        return;
    }
    let is_cross_join = str_field(node, "jointype") == Some("JOIN_INNER")
        && field(node, "quals").is_none()
        && field(node, "usingClause").is_none()
        && !node
            .get("isNatural")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    if is_cross_join {
        ctx.report(node, "noCrossJoin");
    }
}
