//! Port of `require-primary-key`: every `CREATE TABLE` should have a PRIMARY
//! KEY (either a column constraint or a table-level constraint).

use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

fn has_primary_key(elts: &[Value]) -> bool {
    for elt in elts {
        // Table-level PRIMARY KEY constraint
        if is_type(elt, "Constraint") && str_field(elt, "contype") == Some("CONSTR_PRIMARY") {
            return true;
        }
        // Column-level PRIMARY KEY constraint (inside a ColumnDef)
        if is_type(elt, "ColumnDef")
            && let Some(constraints) = array_field(elt, "constraints")
        {
            for constraint in constraints {
                if is_type(constraint, "Constraint")
                    && str_field(constraint, "contype") == Some("CONSTR_PRIMARY")
                {
                    return true;
                }
            }
        }
    }
    false
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let elts = match array_field(node, "tableElts") {
        Some(e) if !e.is_empty() => e,
        _ => return,
    };
    if has_primary_key(elts) {
        return;
    }
    let relname = field(node, "relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    ctx.report_data(
        node,
        "missingPrimaryKey",
        SmallVec::from_iter([DiagnosticDatum {
            key: CompactString::from("table"),
            value: CompactString::from(relname),
        }]),
    );
}
