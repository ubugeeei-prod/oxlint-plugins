//! Port of `no-select-into`: disallow `SELECT ... INTO target FROM ...` (which
//! creates a new table). Use `CREATE TABLE target AS SELECT ...` instead.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    if field(node, "intoClause").is_none() {
        return;
    }
    ctx.report(node, "noSelectInto");
}
