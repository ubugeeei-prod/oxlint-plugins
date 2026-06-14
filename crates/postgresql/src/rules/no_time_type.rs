//! Port of `no-time-type`: disallow `TIME` / `TIME WITH TIME ZONE` columns.
//! `time` has no date (cannot disambiguate around DST) and `timetz` stores an
//! offset that is meaningless without a date. Reports the `ColumnDef` whose
//! type resolves to `time` or `timetz`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, type_name};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    if matches!(type_name(field(node, "typeName")), Some("time" | "timetz")) {
        ctx.report(node, "noTimeType");
    }
}
