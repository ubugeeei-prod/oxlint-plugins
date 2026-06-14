//! Port of `no-alter-column-type`: disallow
//! `ALTER TABLE ... ALTER COLUMN ... TYPE ...` because it can rewrite the table
//! under an ACCESS EXCLUSIVE lock.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "AlterTableCmd") && str_field(node, "subtype") == Some("AT_AlterColumnType") {
        ctx.report(node, "noAlterColumnType");
    }
}
