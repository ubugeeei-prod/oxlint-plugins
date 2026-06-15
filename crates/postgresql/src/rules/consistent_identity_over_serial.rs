//! Port of `consistent-identity-over-serial`: enforce a consistent stance on
//! `GENERATED ... AS IDENTITY` vs `SERIAL` / `BIGSERIAL` / `SMALLSERIAL`.
//!
//! Default style `"always"` flags any column typed as a serial pseudo-type and
//! requires the SQL-standard identity column form. Style `"never"` flags any
//! column with a `CONSTR_IDENTITY` constraint and requires a serial type.
//!
//! Reports on the `ColumnDef` node (the column definition itself).

use serde_json::Value;

use crate::ast::{get_type_name, is_type};
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

const SERIAL_TYPES: [&str; 3] = ["smallserial", "serial", "bigserial"];

/// Returns true when the ColumnDef has any `CONSTR_IDENTITY` constraint
/// (mirrors upstream `hasIdentity`).
fn has_identity(node: &Value) -> bool {
    let constraints = match node.get("constraints").and_then(Value::as_array) {
        Some(c) => c,
        None => return false,
    };
    constraints.iter().any(|c| {
        is_type(c, "Constraint")
            && c.get("contype").and_then(Value::as_str) == Some("CONSTR_IDENTITY")
    })
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }

    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");

    if style == "always" {
        let type_name = match node.get("typeName") {
            Some(t) => t,
            None => return,
        };
        let t = match get_type_name(type_name) {
            Some(n) => n,
            None => return,
        };
        if SERIAL_TYPES.contains(&t) {
            let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
            data.push(DiagnosticDatum {
                key: CompactString::from("type"),
                value: CompactString::from(t),
            });
            ctx.report_data(node, "preferIdentity", data);
        }
    } else if style == "never" && has_identity(node) {
        ctx.report(node, "unexpectedIdentity");
    }
}
