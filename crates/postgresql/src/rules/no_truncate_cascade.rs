//! Port of `no-truncate-cascade`: disallow `TRUNCATE ... CASCADE` because it
//! transitively empties referencing tables.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "TruncateStmt") && str_field(node, "behavior") == Some("DROP_CASCADE") {
        ctx.report(node, "noCascade");
    }
}
