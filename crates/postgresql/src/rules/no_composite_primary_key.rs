//! Port of `no-composite-primary-key`: disallow composite (multi-column)
//! PRIMARY KEY constraints. Mirrors upstream's two visitors: a table-level
//! `Constraint` inside `CREATE TABLE (... PRIMARY KEY (a, b))` is reported at
//! the constraint node, while `ALTER TABLE ... ADD CONSTRAINT ... PRIMARY KEY
//! (a, b)` is reported at the `AlterTableCmd`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

fn is_composite_primary_key(def: &Value) -> bool {
    is_type(def, "Constraint")
        && str_field(def, "contype") == Some("CONSTR_PRIMARY")
        && array_field(def, "keys").is_some_and(|keys| keys.len() > 1)
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "CreateStmt") {
        if let Some(elts) = array_field(node, "tableElts") {
            for elt in elts {
                if is_composite_primary_key(elt) {
                    ctx.report(elt, "noCompositePk");
                }
            }
        }
        return;
    }
    if is_type(node, "AlterTableCmd")
        && str_field(node, "subtype") == Some("AT_AddConstraint")
        && field(node, "def").is_some_and(is_composite_primary_key)
    {
        ctx.report(node, "noCompositePk");
    }
}
