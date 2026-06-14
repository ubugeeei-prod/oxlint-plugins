//! Port of `no-rename-column`: disallow `ALTER TABLE ... RENAME COLUMN` —
//! every deployed reader of the old name breaks at deploy time.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "RenameStmt") && str_field(node, "renameType") == Some("OBJECT_COLUMN") {
        ctx.report(node, "noRenameColumn");
    }
}
