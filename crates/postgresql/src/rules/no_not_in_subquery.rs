//! Port of `no-not-in-subquery`: disallow `NOT IN (subquery)` because it
//! returns no rows when the subquery yields any NULL — use `NOT EXISTS` instead.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "BoolExpr") {
        return;
    }
    if str_field(node, "boolop") != Some("NOT_EXPR") {
        return;
    }
    let args = match array_field(node, "args") {
        Some(a) if a.len() == 1 => a,
        _ => return,
    };
    let sub = &args[0];
    if !is_type(sub, "SubLink") {
        return;
    }
    if str_field(sub, "subLinkType") != Some("ANY_SUBLINK") {
        return;
    }
    // operName must be absent or null (plain IN, not a custom operator)
    if field(sub, "operName").is_some_and(|v| !v.is_null()) {
        return;
    }
    // testexpr must be present and non-null
    if field(sub, "testexpr").is_none_or(|v| v.is_null()) {
        return;
    }
    ctx.report(node, "noNotInSubquery");
}
