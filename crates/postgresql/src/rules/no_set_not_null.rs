//! Port of `no-set-not-null`: disallow `ALTER COLUMN ... SET NOT NULL` because
//! it scans the whole table under ACCESS EXCLUSIVE.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "AlterTableCmd") && str_field(node, "subtype") == Some("AT_SetNotNull") {
        ctx.report(node, "noSetNotNull");
    }
}
