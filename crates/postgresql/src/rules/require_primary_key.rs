//! Port of `require-primary-key`: require every `CREATE TABLE` to declare a
//! primary key, either as a column constraint or a table-level constraint.

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::ast::{array_field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};

fn has_primary_key(elts: &[Value]) -> bool {
    elts.iter().any(|elt| {
        if is_type(elt, "Constraint") && str_field(elt, "contype") == Some("CONSTR_PRIMARY") {
            return true;
        }
        if is_type(elt, "ColumnDef")
            && let Some(constraints) = array_field(elt, "constraints")
        {
            return constraints.iter().any(|c| {
                is_type(c, "Constraint") && str_field(c, "contype") == Some("CONSTR_PRIMARY")
            });
        }
        false
    })
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    // No tableElts means CREATE TABLE ... PARTITION OF or similar; skip.
    let Some(elts) = array_field(node, "tableElts") else {
        return;
    };
    if elts.is_empty() {
        return;
    }
    if has_primary_key(elts) {
        return;
    }
    let relname = node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("table"),
        value: CompactString::from(relname),
    });
    ctx.report_data(node, "missingPrimaryKey", data);
}
