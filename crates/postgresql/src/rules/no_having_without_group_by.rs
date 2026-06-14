//! Port of `no-having-without-group-by`: disallow `HAVING` without `GROUP BY`
//! — the query aggregates the entire result set, which is almost never the
//! intended shape.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    if field(node, "havingClause").is_none() {
        return;
    }
    // Allow when groupClause is present and non-empty.
    if array_field(node, "groupClause").is_some_and(|g| !g.is_empty()) {
        return;
    }
    ctx.report(node, "noHavingWithoutGroupBy");
}
