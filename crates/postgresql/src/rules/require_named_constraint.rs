//! Port of `require-named-constraint`: require an explicit name on table-level
//! CHECK / UNIQUE / FOREIGN KEY / EXCLUSION constraints, whether declared in
//! `CREATE TABLE (..., <constraint>)` or via `ALTER TABLE ... ADD ...`.
//! Column-level constraints (nested in a ColumnDef) are out of scope, so the
//! rule matches only the CreateStmt table-element list and the
//! AT_AddConstraint command, mirroring upstream.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

const NAMED_CONSTRAINT_TYPES: &[&str] = &[
    "CONSTR_CHECK",
    "CONSTR_UNIQUE",
    "CONSTR_FOREIGN",
    "CONSTR_EXCLUSION",
];

fn is_unnamed_named_kind(c: &Value) -> bool {
    let Some(contype) = str_field(c, "contype") else {
        return false;
    };
    if !NAMED_CONSTRAINT_TYPES.contains(&contype) {
        return false;
    }
    // Unnamed when `conname` is absent or empty.
    str_field(c, "conname").is_none_or(str::is_empty)
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "CreateStmt") {
        let Some(elts) = array_field(node, "tableElts") else {
            return;
        };
        for elt in elts {
            if is_type(elt, "Constraint") && is_unnamed_named_kind(elt) {
                ctx.report(elt, "requireNamedConstraint");
            }
        }
        return;
    }
    if is_type(node, "AlterTableCmd") && str_field(node, "subtype") == Some("AT_AddConstraint") {
        let Some(def) = field(node, "def") else {
            return;
        };
        if is_type(def, "Constraint") && is_unnamed_named_kind(def) {
            ctx.report(node, "requireNamedConstraint");
        }
    }
}
