//! Port of `no-add-check-constraint-without-not-valid`: disallow
//! `ALTER TABLE ... ADD CONSTRAINT ... CHECK (...)` without `NOT VALID`.
//! The synchronous form holds ACCESS EXCLUSIVE for the entire validating scan.

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
    if str_field(def, "contype") != Some("CONSTR_CHECK") {
        return;
    }
    // `skip_validation == true` means `NOT VALID` was supplied → allowed.
    if def.get("skip_validation") == Some(&Value::Bool(true)) {
        return;
    }
    // Report the `AlterTableCmd`, exactly like upstream; its computed span
    // already covers the `CONSTRAINT … CHECK (…)` subtree.
    ctx.report(node, "checkNotValid");
}
