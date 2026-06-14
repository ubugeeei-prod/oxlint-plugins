//! Port of `no-drop-schema-cascade`: disallow `DROP SCHEMA ... CASCADE`
//! because it silently removes every object in the schema.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "DropStmt") {
        return;
    }
    if str_field(node, "removeType") != Some("OBJECT_SCHEMA") {
        return;
    }
    if str_field(node, "behavior") != Some("DROP_CASCADE") {
        return;
    }
    ctx.report(node, "noDropSchemaCascade");
}
