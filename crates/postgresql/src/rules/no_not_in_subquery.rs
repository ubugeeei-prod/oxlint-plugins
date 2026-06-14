//! Port of `no-not-in-subquery`: disallow `NOT IN (subquery)` because NULLs in
//! the subquery silently return zero rows.
//!
//! libpg_query encodes `NOT IN (subq)` as a `BoolExpr` with `boolop = NOT_EXPR`
//! whose single arg is a `SubLink` of `ANY_SUBLINK` kind, with a `testexpr` and
//! no `operName` (the `= ANY(...)` form sets `operName`).

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
    let Some(args) = array_field(node, "args") else {
        return;
    };
    if args.len() != 1 {
        return;
    }
    let arg = &args[0];
    if !is_type(arg, "SubLink") {
        return;
    }
    if str_field(arg, "subLinkType") != Some("ANY_SUBLINK") {
        return;
    }
    if field(arg, "operName").is_some() {
        return;
    }
    if field(arg, "testexpr").is_none() {
        return;
    }
    ctx.report(node, "noNotInSubquery");
}
