//! Port of `prefer-bigint-id`: prefer `bigint` for primary-key `id` columns.
//! Flags an `id` column whose type is a 32-bit-or-smaller integer (`int2`,
//! `int4`, `smallserial`, `serial`) when it is the primary key, either through a
//! column constraint or a table-level `PRIMARY KEY (id)`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field, type_name};

fn is_primary_key(col: &Value) -> bool {
    array_field(col, "constraints").is_some_and(|cs| {
        cs.iter()
            .any(|c| is_type(c, "Constraint") && str_field(c, "contype") == Some("CONSTR_PRIMARY"))
    })
}

fn table_primary_key_on(elts: &[Value], colname: &str) -> bool {
    elts.iter().any(|elt| {
        is_type(elt, "Constraint")
            && str_field(elt, "contype") == Some("CONSTR_PRIMARY")
            && array_field(elt, "keys").is_some_and(|keys| {
                keys.iter()
                    .any(|k| k.get("sval").and_then(Value::as_str) == Some(colname))
            })
    })
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let Some(elts) = array_field(node, "tableElts") else {
        return;
    };
    for elt in elts {
        if !is_type(elt, "ColumnDef") {
            continue;
        }
        if str_field(elt, "colname") != Some("id") {
            continue;
        }
        let Some(t) = type_name(field(elt, "typeName")) else {
            continue;
        };
        if !matches!(t, "int2" | "int4" | "smallserial" | "serial") {
            continue;
        }
        if !is_primary_key(elt) && !table_primary_key_on(elts, "id") {
            continue;
        }
        ctx.report(elt, "preferBigintId");
    }
}
