//! Port of `no-composite-primary-key`: disallow composite PRIMARY KEY
//! constraints (more than one column). Use a single-column surrogate key and
//! a UNIQUE constraint for the natural key instead.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

fn is_composite_primary_key(elt: &Value) -> bool {
    if !is_type(elt, "Constraint") {
        return false;
    }
    if str_field(elt, "contype") != Some("CONSTR_PRIMARY") {
        return false;
    }
    array_field(elt, "keys").is_some_and(|k| k.len() > 1)
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "CreateStmt") {
        let elts = match array_field(node, "tableElts") {
            Some(e) => e,
            None => return,
        };
        for elt in elts {
            if is_composite_primary_key(elt) {
                ctx.report(elt, "noCompositePk");
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
        if is_composite_primary_key(def) {
            ctx.report(node, "noCompositePk");
        }
    }
}
