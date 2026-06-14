//! Port of `no-distinct-on-without-order-by`: disallow `SELECT DISTINCT ON (...)`
//! without a matching `ORDER BY`. Without `ORDER BY`, the surviving row in each
//! distinct group is non-deterministic (PostgreSQL picks one arbitrarily).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type};

/// A `SelectStmt` has `DISTINCT ON` (as opposed to plain `DISTINCT`) when its
/// `distinctClause` array is non-empty **and** at least one element carries a
/// `type` field. Plain `DISTINCT` (no column list) emits a single element with
/// no `type`.
fn has_distinct_on(node: &Value) -> bool {
    let Some(distinct_clause) = array_field(node, "distinctClause") else {
        return false;
    };
    distinct_clause.iter().any(|e| e.get("type").is_some())
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    if !has_distinct_on(node) {
        return;
    }
    // Allow if there is at least one sort clause.
    if array_field(node, "sortClause").is_some_and(|s| !s.is_empty()) {
        return;
    }
    ctx.report(node, "noDistinctOnWithoutOrderBy");
}
