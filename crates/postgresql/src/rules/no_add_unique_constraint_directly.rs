//! Port of `no-add-unique-constraint-directly`: disallow
//! `ALTER TABLE ... ADD CONSTRAINT ... UNIQUE (...)` written inline. The
//! `USING INDEX <name>` form populates the constraint's `indexname`; its
//! absence means the user wrote the inline form that builds the index
//! synchronously under `ACCESS EXCLUSIVE`.

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
    if str_field(def, "contype") != Some("CONSTR_UNIQUE") {
        return;
    }
    if str_field(def, "indexname").is_some() {
        return;
    }
    ctx.report(node, "useIndexFirst");
}
