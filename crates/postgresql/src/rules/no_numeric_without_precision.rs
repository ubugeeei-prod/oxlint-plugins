//! Port of `no-numeric-without-precision`: require an explicit precision (and
//! scale) on `NUMERIC` / `DECIMAL` columns. A bare `numeric` accepts unbounded
//! magnitude and encodes nothing about the column's domain.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

/// Mirrors upstream `getTypeName` (`src/utils/ast.ts`): the canonical type name
/// is the segment at key `"1"` (lower segments are schema qualifiers such as
/// `pg_catalog`), falling back to key `"0"` for unqualified names.
fn get_type_name(type_name: &Value) -> Option<&str> {
    if !type_name.is_object() {
        return None;
    }
    if let Some(v1) = type_name
        .get("1")
        .and_then(|s| s.get("sval"))
        .and_then(Value::as_str)
    {
        return Some(v1);
    }
    type_name
        .get("0")
        .and_then(|s| s.get("sval"))
        .and_then(Value::as_str)
}

/// Mirrors upstream `hasTypmods`: true when `typeName.typmods` is a non-empty
/// array (i.e. a precision/scale was declared).
fn has_typmods(type_name: &Value) -> bool {
    type_name
        .get("typmods")
        .and_then(Value::as_array)
        .is_some_and(|mods| !mods.is_empty())
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    let Some(type_name) = node.get("typeName") else {
        return;
    };
    if get_type_name(type_name) != Some("numeric") {
        return;
    }
    if has_typmods(type_name) {
        return;
    }
    ctx.report(node, "noNumericWithoutPrecision");
}
