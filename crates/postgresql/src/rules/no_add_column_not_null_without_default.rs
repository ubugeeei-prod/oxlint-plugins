//! Port of `no-add-column-not-null-without-default`: disallow
//! `ALTER TABLE ADD COLUMN ... NOT NULL` without a `DEFAULT` because
//! the migration fails outright on any non-empty table.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

fn has_not_null(def: &Value) -> bool {
    let Some(constraints) = array_field(def, "constraints") else {
        return false;
    };
    constraints
        .iter()
        .any(|c| is_type(c, "Constraint") && str_field(c, "contype") == Some("CONSTR_NOTNULL"))
}

fn has_default(def: &Value) -> bool {
    let Some(constraints) = array_field(def, "constraints") else {
        return false;
    };
    constraints.iter().any(|c| {
        if !is_type(c, "Constraint") {
            return false;
        }
        matches!(
            str_field(c, "contype"),
            Some("CONSTR_DEFAULT") | Some("CONSTR_GENERATED") | Some("CONSTR_IDENTITY")
        )
    })
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
    if !is_type(def, "ColumnDef") {
        return;
    }
    if !has_not_null(def) {
        return;
    }
    if has_default(def) {
        return;
    }
    ctx.report(node, "noAddColumnNotNullWithoutDefault");
}
