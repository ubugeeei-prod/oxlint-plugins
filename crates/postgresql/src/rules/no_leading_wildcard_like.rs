//! Port of `no-leading-wildcard-like`: disallow LIKE/ILIKE patterns starting
//! with `%` because they cannot use a B-tree index and force a full scan.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, str_field};

fn get_string_const(node: &Value) -> Option<&str> {
    if !is_type(node, "A_Const") {
        return None;
    }
    // libpg_query encodes string constants as A_Const { sval: { sval: "..." } }
    node.get("sval")?.get("sval")?.as_str()
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "A_Expr") {
        return;
    }
    let kind = str_field(node, "kind");
    if kind != Some("AEXPR_LIKE") && kind != Some("AEXPR_ILIKE") {
        return;
    }
    let Some(rexpr) = field(node, "rexpr") else {
        return;
    };
    let Some(pattern) = get_string_const(rexpr) else {
        return;
    };
    if !pattern.starts_with('%') {
        return;
    }
    ctx.report(node, "noLeadingWildcardLike");
}
