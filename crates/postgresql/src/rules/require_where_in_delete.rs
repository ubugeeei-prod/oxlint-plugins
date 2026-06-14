//! Port of `require-where-in-delete`: require a WHERE clause in DELETE
//! statements so that full-table deletes are always intentional.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "DeleteStmt") && field(node, "whereClause").is_none() {
        ctx.report(node, "missingWhere");
    }
}
