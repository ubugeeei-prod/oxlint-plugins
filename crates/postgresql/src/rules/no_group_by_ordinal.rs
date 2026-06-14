//! Port of `no-group-by-ordinal`: disallow `GROUP BY <position>` (ordinal
//! references); use column names or expressions instead.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type};

fn is_integer_const(node: &Value) -> bool {
    if !is_type(node, "A_Const") {
        return false;
    }
    matches!(node.get("ival"), Some(v) if v.is_object())
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    let Some(group_clause) = array_field(node, "groupClause") else {
        return;
    };
    for expr in group_clause {
        if is_integer_const(expr) {
            ctx.report(expr, "noGroupByOrdinal");
        }
    }
}
