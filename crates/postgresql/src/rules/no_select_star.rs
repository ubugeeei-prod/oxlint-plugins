//! Port of `no-select-star`: disallow `SELECT *` (and `t.*`) so result schemas
//! stay stable when the underlying table changes.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    let Some(target_list) = array_field(node, "targetList") else {
        return;
    };
    for target in target_list {
        if !is_type(target, "ResTarget") {
            continue;
        }
        let Some(val) = field(target, "val") else {
            continue;
        };
        if !is_type(val, "ColumnRef") {
            continue;
        }
        let Some(fields) = array_field(val, "fields") else {
            continue;
        };
        if fields.iter().any(|f| is_type(f, "A_Star")) {
            ctx.report(target, "noSelectStar");
        }
    }
}
