//! Port of `consistent-fk-not-valid`
use crate::RuleContext;
use crate::ast::{field, is_type, str_field};
use serde_json::Value;

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
    if str_field(def, "contype") != Some("CONSTR_FOREIGN") {
        return;
    }
    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");
    let skip_validation = def
        .get("skip_validation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if style == "always" && !skip_validation {
        ctx.report(node, "preferFkNotValid");
    } else if style == "never" && skip_validation {
        ctx.report(node, "unexpectedFkNotValid");
    }
}
