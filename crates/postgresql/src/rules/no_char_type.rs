//! Port of `no-char-type`: disallow the blank-padded `char(n)` / `bpchar`
//! column type. PostgreSQL pads stored values to `n` with trailing spaces and
//! trims on read, surprising every comparison and round-trip.

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

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    let Some(type_name) = node.get("typeName") else {
        return;
    };
    if get_type_name(type_name) == Some("bpchar") {
        ctx.report(node, "noChar");
    }
}
