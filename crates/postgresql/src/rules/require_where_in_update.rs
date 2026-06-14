//! Port of `require-where-in-update`: require a WHERE clause in UPDATE
//! statements so that full-table updates are always intentional.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "UpdateStmt") && field(node, "whereClause").is_none() {
        ctx.report(node, "missingWhere");
    }
}
