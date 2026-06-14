//! Port of `no-drop-column`: disallow `ALTER TABLE ... DROP COLUMN` because
//! every reader of the dropped column breaks at deploy time.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "AlterTableCmd") && str_field(node, "subtype") == Some("AT_DropColumn") {
        ctx.report(node, "noDropColumn");
    }
}
