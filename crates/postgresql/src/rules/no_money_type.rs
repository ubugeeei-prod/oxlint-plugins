//! Port of `no-money-type`: disallow the `money` column type. Its output
//! format and precision depend on `lc_monetary`, so the same row looks
//! different on different servers and round-trips badly.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{get_type_name, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    let Some(type_name) = node.get("typeName") else {
        return;
    };
    if get_type_name(type_name) == Some("money") {
        ctx.report(node, "noMoney");
    }
}
