//! Port of `no-char-type`: disallow the blank-padded `char(n)` / `bpchar`
//! column type. PostgreSQL pads stored values to `n` with trailing spaces and
//! trims on read, surprising every comparison and round-trip.

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
    if get_type_name(type_name) == Some("bpchar") {
        ctx.report(node, "noChar");
    }
}
