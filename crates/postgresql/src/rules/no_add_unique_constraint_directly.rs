//! Port of `no-add-unique-constraint-directly`: disallow
//! `ALTER TABLE ... ADD CONSTRAINT ... UNIQUE (...)` without building the
//! index first via `CREATE UNIQUE INDEX CONCURRENTLY` then promoting it.
//! The inline form holds ACCESS EXCLUSIVE for the entire index build.

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
    // `indexname` is set when `UNIQUE USING INDEX <name>` is used (the safe
    // pattern — the index was already built out-of-band with CONCURRENTLY).
    // Its absence means the constraint builds the index inline; report that.
    if def.get("indexname").is_some_and(|v| !v.is_null()) {
        return;
    }
    // Report the `AlterTableCmd`, exactly like upstream.
    ctx.report(node, "useIndexFirst");
}
