//! Port of `no-time-type`: disallow `TIME` / `TIME WITH TIME ZONE` (`timetz`)
//! columns. `time` has no date so cannot disambiguate around DST, and `timetz`
//! stores an offset that is meaningless without a date.

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
    // Upstream's `TIME_TYPES` set; `getTypeName` must return a string.
    let Some(t) = get_type_name(type_name) else {
        return;
    };
    if t == "time" || t == "timetz" {
        ctx.report(node, "noTimeType");
    }
}
