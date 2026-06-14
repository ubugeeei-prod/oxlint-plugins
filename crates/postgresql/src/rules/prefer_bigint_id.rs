//! Port of `prefer-bigint-id`: primary-key `id` columns should be `bigint`.
//! `int` / `smallint` (and the `serial` / `smallserial` pseudo-types) overflow
//! on growing tables and widening them later forces a table rewrite under
//! `ACCESS EXCLUSIVE`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{get_type_name, is_type};

/// Upstream `SMALL_INT_TYPES`: canonical small-integer type names plus the
/// pseudo-types PostgreSQL keeps before a SERIAL is rewritten.
fn is_small_int_type(name: &str) -> bool {
    matches!(name, "int2" | "int4" | "smallserial" | "serial")
}

/// Mirrors upstream `isPrimaryKey`: a column-level `CONSTR_PRIMARY` constraint.
fn is_primary_key(col: &Value) -> bool {
    col.get("constraints")
        .and_then(Value::as_array)
        .is_some_and(|cs| {
            cs.iter().any(|c| {
                is_type(c, "Constraint")
                    && c.get("contype").and_then(Value::as_str) == Some("CONSTR_PRIMARY")
            })
        })
}

/// Mirrors upstream `isTablePrimaryKeyOn`: a table-level `CONSTR_PRIMARY`
/// constraint whose `keys` include `colname`.
fn is_table_primary_key_on(elts: &[Value], colname: &str) -> bool {
    elts.iter().any(|elt| {
        if !is_type(elt, "Constraint") {
            return false;
        }
        if elt.get("contype").and_then(Value::as_str) != Some("CONSTR_PRIMARY") {
            return false;
        }
        elt.get("keys")
            .and_then(Value::as_array)
            .is_some_and(|keys| {
                keys.iter()
                    .any(|k| k.get("sval").and_then(Value::as_str) == Some(colname))
            })
    })
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let Some(elts) = node.get("tableElts").and_then(Value::as_array) else {
        return;
    };
    for elt in elts {
        if !is_type(elt, "ColumnDef") {
            continue;
        }
        if elt.get("colname").and_then(Value::as_str) != Some("id") {
            continue;
        }
        let Some(type_name) = elt.get("typeName") else {
            continue;
        };
        let Some(t) = get_type_name(type_name) else {
            continue;
        };
        if !is_small_int_type(t) {
            continue;
        }
        if !is_primary_key(elt) && !is_table_primary_key_on(elts, "id") {
            continue;
        }
        ctx.report(elt, "preferBigintId");
    }
}
