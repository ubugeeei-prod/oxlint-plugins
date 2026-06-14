//! Port of `no-update-without-from-binding`: disallow `UPDATE ... FROM other`
//! without a `WHERE` clause (a Cartesian product with the target table).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "UpdateStmt") {
        return;
    }
    let has_from = array_field(node, "fromClause").is_some_and(|f| !f.is_empty());
    if !has_from {
        return;
    }
    if field(node, "whereClause").is_some() {
        return;
    }
    ctx.report(node, "missingJoin");
}
