//! Port of `prefer-add-constraint-not-valid`: prefer
//! `ALTER TABLE ... ADD CONSTRAINT ... NOT VALID` followed by a separate
//! `VALIDATE CONSTRAINT`. FOREIGN KEY and CHECK constraints scan existing rows
//! to validate; adding them with `NOT VALID` skips that scan under
//! `ACCESS EXCLUSIVE`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

/// Upstream `VALIDATING_CONTYPES`: the constraint kinds whose validating scan
/// benefits from `NOT VALID`.
fn is_validating_contype(contype: &str) -> bool {
    matches!(contype, "CONSTR_FOREIGN" | "CONSTR_CHECK")
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "AlterTableCmd") {
        return;
    }
    if node.get("subtype").and_then(Value::as_str) != Some("AT_AddConstraint") {
        return;
    }
    let Some(def) = node.get("def") else {
        return;
    };
    if !is_type(def, "Constraint") {
        return;
    }
    // `!contype || !VALIDATING_CONTYPES.has(contype)`: a missing/empty or
    // non-validating contype is not flagged.
    match def.get("contype").and_then(Value::as_str) {
        Some(c) if is_validating_contype(c) => {}
        _ => return,
    }
    // `skip_validation === true` means the user already wrote `NOT VALID`.
    if def.get("skip_validation") == Some(&Value::Bool(true)) {
        return;
    }
    ctx.report(node, "notValid");
}
