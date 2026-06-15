//! Port of `no-update-without-from-binding`: disallow `UPDATE ... FROM ...`
//! without a `WHERE` clause, which produces a Cartesian product.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "UpdateStmt") {
        return;
    }
    // Must have a FROM clause with at least one element
    if array_field(node, "fromClause").is_none_or(|f| f.is_empty()) {
        return;
    }
    // Must be missing a WHERE clause
    if field(node, "whereClause").is_some() {
        return;
    }
    ctx.report(node, "missingJoin");
}
