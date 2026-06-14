//! Port of `consistent-fk-not-valid`: enforce a consistent stance on
//! `NOT VALID` for `ALTER TABLE ... ADD FOREIGN KEY`. `NOT VALID` sets
//! `skip_validation: true`; option `style` is `always` (default, require it) or
//! `never` (forbid it).

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
    if str_field(def, "contype") != Some("CONSTR_FOREIGN") {
        return;
    }
    let skips = def.get("skip_validation") == Some(&Value::Bool(true));
    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");
    if style == "always" && !skips {
        ctx.report(node, "preferFkNotValid");
    } else if style == "never" && skips {
        ctx.report(node, "unexpectedFkNotValid");
    }
}
