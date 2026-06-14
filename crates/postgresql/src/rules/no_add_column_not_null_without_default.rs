//! Port of `no-add-column-not-null-without-default`: disallow
//! `ALTER TABLE ADD COLUMN ... NOT NULL` without a `DEFAULT` because the
//! migration fails outright on any non-empty table. A DEFAULT, GENERATED, or
//! IDENTITY constraint on the new column makes it safe.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

fn has_any_contype(constraints: &[Value], wanted: &[&str]) -> bool {
    constraints.iter().any(|c| {
        is_type(c, "Constraint") && str_field(c, "contype").is_some_and(|t| wanted.contains(&t))
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
    let Some(constraints) = array_field(def, "constraints") else {
        return;
    };
    if !has_any_contype(constraints, &["CONSTR_NOTNULL"]) {
        return;
    }
    if has_any_contype(
        constraints,
        &["CONSTR_DEFAULT", "CONSTR_GENERATED", "CONSTR_IDENTITY"],
    ) {
        return;
    }
    ctx.report(node, "noAddColumnNotNullWithoutDefault");
}
