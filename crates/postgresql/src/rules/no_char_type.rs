//! Port of `no-char-type`: disallow the blank-padded `char(n)` / `bpchar`
//! column type. PostgreSQL pads stored values to `n` with trailing spaces and
//! trims on read, surprising every comparison and round-trip. Reports the
//! `ColumnDef` whose type resolves to `bpchar` (both `CHAR(n)` and `BPCHAR`).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, type_name};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "ColumnDef") && type_name(field(node, "typeName")) == Some("bpchar") {
        ctx.report(node, "noChar");
    }
}
