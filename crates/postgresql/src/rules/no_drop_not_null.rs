//! Port of `no-drop-not-null`: disallow `ALTER COLUMN ... DROP NOT NULL`
//! because relaxing a NOT NULL constraint surprises every consumer that already
//! assumes the column is non-null.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "AlterTableCmd") && str_field(node, "subtype") == Some("AT_DropNotNull") {
        ctx.report(node, "noDropNotNull");
    }
}
