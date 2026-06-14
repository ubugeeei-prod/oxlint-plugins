//! Port of `no-drop-schema-cascade`: disallow `DROP SCHEMA ... CASCADE`, which
//! silently removes every object in the schema.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "DropStmt")
        && str_field(node, "removeType") == Some("OBJECT_SCHEMA")
        && str_field(node, "behavior") == Some("DROP_CASCADE")
    {
        ctx.report(node, "noDropSchemaCascade");
    }
}
