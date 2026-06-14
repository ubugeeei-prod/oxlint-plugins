//! Port of `prefer-add-constraint-not-valid`: prefer
//! `ADD CONSTRAINT ... NOT VALID` + a separate `VALIDATE CONSTRAINT` for the
//! validating constraint kinds (FOREIGN KEY, CHECK) so the validating scan
//! does not hold `ACCESS EXCLUSIVE`. `skip_validation: true` means the user
//! already wrote `NOT VALID`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "AlterTableCmd") {
        return;
    }
    if str_field(node, "subtype") != Some("AT_AddConstraint") {
        return;
    }
    let Some(def) = field(node, "def") else {
        return;
    };
    if !is_type(def, "Constraint") {
        return;
    }
    match str_field(def, "contype") {
        Some("CONSTR_FOREIGN") | Some("CONSTR_CHECK") => {}
        _ => return,
    }
    if def.get("skip_validation") == Some(&Value::Bool(true)) {
        return;
    }
    ctx.report(node, "notValid");
}
