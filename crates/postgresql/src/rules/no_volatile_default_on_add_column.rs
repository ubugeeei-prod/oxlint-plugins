//! Port of `no-volatile-default-on-add-column`: disallow
//! `ALTER TABLE ... ADD COLUMN ... DEFAULT <volatile>()` because volatile
//! defaults force a full table rewrite under ACCESS EXCLUSIVE.
//!
//! Known volatile defaults: random, gen_random_uuid, uuid_generate_v1,
//! uuid_generate_v1mc, uuid_generate_v4, clock_timestamp, timeofday.
//! Stable defaults (now(), current_timestamp) are NOT in the list.

use serde_json::Value;

use crate::ast::{array_field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Functions whose value differs row-to-row when used as a column DEFAULT.
/// Mirrors the upstream `VOLATILE_DEFAULTS` set.
fn is_volatile_name(name: &str) -> bool {
    matches!(
        name,
        "random"
            | "gen_random_uuid"
            | "uuid_generate_v1"
            | "uuid_generate_v1mc"
            | "uuid_generate_v4"
            | "clock_timestamp"
            | "timeofday"
    )
}

/// Ported from upstream `isVolatileDefault`. Unwraps one level of `TypeCast`
/// (e.g. `gen_random_uuid()::uuid`) then checks if the expression is a
/// `FuncCall` whose last `funcname` element matches a volatile function name.
/// Returns the function name if volatile, `None` otherwise.
fn volatile_func_name(raw_expr: &Value) -> Option<&str> {
    // Unwrap TypeCast → check its arg.
    if is_type(raw_expr, "TypeCast") {
        return raw_expr.get("arg").and_then(volatile_func_name);
    }
    if !is_type(raw_expr, "FuncCall") {
        return None;
    }
    let funcname = array_field(raw_expr, "funcname")?;
    // The last element is the unqualified function name; earlier elements are
    // schema qualifiers.
    let last = funcname.last()?;
    let name = last.get("sval").and_then(Value::as_str)?;
    if is_volatile_name(name) {
        Some(name)
    } else {
        None
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "AlterTableCmd") {
        return;
    }
    if str_field(node, "subtype") != Some("AT_AddColumn") {
        return;
    }

    let constraints = match node
        .get("def")
        .and_then(|d| d.get("constraints"))
        .and_then(Value::as_array)
    {
        Some(c) => c,
        None => return,
    };

    for constraint in constraints {
        if !is_type(constraint, "Constraint") {
            continue;
        }
        if str_field(constraint, "contype") != Some("CONSTR_DEFAULT") {
            continue;
        }
        let raw_expr = match constraint.get("raw_expr") {
            Some(e) => e,
            None => continue,
        };
        let fn_name = match volatile_func_name(raw_expr) {
            Some(n) => n,
            None => continue,
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("fn"),
            value: CompactString::from(fn_name),
        });
        ctx.report_data(constraint, "noVolatileDefault", data);
    }
}
