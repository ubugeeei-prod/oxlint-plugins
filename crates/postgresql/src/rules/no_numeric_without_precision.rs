//! Port of `no-numeric-without-precision`: require an explicit precision (and
//! scale) on `NUMERIC` / `DECIMAL` columns. A bare `numeric` accepts unbounded
//! magnitude and encodes nothing about the column's domain. Reports the
//! `ColumnDef` whose type resolves to `numeric` and carries no typmods.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, type_name};

fn has_typmods(type_name_node: Option<&Value>) -> bool {
    type_name_node
        .and_then(|t| array_field(t, "typmods"))
        .is_some_and(|mods| !mods.is_empty())
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    let type_name_node = field(node, "typeName");
    if type_name(type_name_node) != Some("numeric") {
        return;
    }
    if has_typmods(type_name_node) {
        return;
    }
    ctx.report(node, "noNumericWithoutPrecision");
}
