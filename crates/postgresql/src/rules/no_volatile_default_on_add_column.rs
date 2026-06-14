//! Port of `no-volatile-default-on-add-column`: disallow
//! `ALTER TABLE ... ADD COLUMN ... DEFAULT <volatile>()`; volatile defaults
//! force a full table rewrite under `ACCESS EXCLUSIVE`. `now()` /
//! `current_timestamp` / `current_date` are STABLE and excluded.

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{array_field, field, is_type, node_type, str_field};
use crate::{DiagnosticDatum, RuleContext};

const VOLATILE_DEFAULTS: &[&str] = &[
    "random",
    "gen_random_uuid",
    "uuid_generate_v1",
    "uuid_generate_v1mc",
    "uuid_generate_v4",
    "clock_timestamp",
    "timeofday",
];

/// Returns the unqualified volatile function name when `raw_expr` is a volatile
/// FuncCall (optionally wrapped in a single TypeCast, e.g. `gen_random_uuid()::uuid`).
fn volatile_default_name(raw_expr: &Value) -> Option<&str> {
    match node_type(raw_expr) {
        Some("TypeCast") => raw_expr.get("arg").and_then(volatile_default_name),
        Some("FuncCall") => {
            let funcname = raw_expr.get("funcname")?.as_array()?;
            let name = funcname.last()?.get("sval")?.as_str()?;
            if VOLATILE_DEFAULTS.contains(&name) {
                Some(name)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "AlterTableCmd") {
        return;
    }
    if str_field(node, "subtype") != Some("AT_AddColumn") {
        return;
    }
    let Some(def) = field(node, "def") else {
        return;
    };
    let Some(constraints) = array_field(def, "constraints") else {
        return;
    };
    for c in constraints {
        if !is_type(c, "Constraint") {
            continue;
        }
        if str_field(c, "contype") != Some("CONSTR_DEFAULT") {
            continue;
        }
        let Some(raw_expr) = c.get("raw_expr") else {
            continue;
        };
        let Some(name) = volatile_default_name(raw_expr) else {
            continue;
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("fn"),
            value: CompactString::from(name),
        });
        ctx.report_data(c, "noVolatileDefault", data);
    }
}
