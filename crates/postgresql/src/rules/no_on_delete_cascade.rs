//! Port of `no-on-delete-cascade`: disallow `ON DELETE CASCADE` on foreign
//! keys. Cascading deletes can silently wipe far more rows than intended.
//! libpg_query encodes `ON DELETE CASCADE` as `fk_del_action: "c"` on a
//! `Constraint` node with `contype: "CONSTR_FOREIGN"`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "Constraint") {
        return;
    }
    if str_field(node, "contype") != Some("CONSTR_FOREIGN") {
        return;
    }
    if str_field(node, "fk_del_action") == Some("c") {
        ctx.report(node, "noCascade");
    }
}
