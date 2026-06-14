//! Port of `no-order-by-ordinal`: disallow `ORDER BY <position>` (ordinal
//! references); use column names or aliases instead.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type};

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
    let Some(sort_clause) = array_field(node, "sortClause") else {
        return;
    };
    for sort_by in sort_clause {
        if !is_type(sort_by, "SortBy") {
            continue;
        }
        let Some(inner) = field(sort_by, "node") else {
            continue;
        };
        if is_integer_const(inner) {
            ctx.report(sort_by, "noOrderByOrdinal");
        }
    }
}
