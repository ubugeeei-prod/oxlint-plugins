//! Port of `require-named-constraint`: require an explicit `CONSTRAINT <name>`
//! on table-level CHECK / UNIQUE / FOREIGN KEY / EXCLUSION constraints.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

const UNNAMED_CONTYPES: &[&str] = &[
    "CONSTR_CHECK",
    "CONSTR_UNIQUE",
    "CONSTR_FOREIGN",
    "CONSTR_EXCLUSION",
];

fn is_unnamed_constraint(elt: &Value) -> bool {
    if !is_type(elt, "Constraint") {
        return false;
    }
    let contype = match str_field(elt, "contype") {
        Some(ct) => ct,
        None => return false,
    };
    if !UNNAMED_CONTYPES.contains(&contype) {
        return false;
    }
    // conname must be absent or an empty string
    match field(elt, "conname") {
        None => true,
        Some(v) => v.as_str().is_some_and(|s| s.is_empty()),
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "CreateStmt") {
        let elts = match array_field(node, "tableElts") {
            Some(e) => e,
            None => return,
        };
        for elt in elts {
            if is_unnamed_constraint(elt) {
                ctx.report(elt, "requireNamedConstraint");
            }
        }
        return;
    }

    if is_type(node, "AlterTableCmd") {
        if str_field(node, "subtype") != Some("AT_AddConstraint") {
            return;
        }
        let def = match field(node, "def") {
            Some(d) => d,
            None => return,
        };
        if is_unnamed_constraint(def) {
            ctx.report(node, "requireNamedConstraint");
        }
    }
}
