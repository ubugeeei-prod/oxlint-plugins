//! Port of `no-drop-table-cascade`: disallow `DROP TABLE ... CASCADE`, which
//! silently removes dependent objects (views, foreign keys, sequences).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "DropStmt")
        && str_field(node, "removeType") == Some("OBJECT_TABLE")
        && str_field(node, "behavior") == Some("DROP_CASCADE")
    {
        ctx.report(node, "noCascade");
    }
}
