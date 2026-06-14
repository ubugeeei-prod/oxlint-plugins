//! Port of `no-drop-database`: disallow `DROP DATABASE` because it is
//! catastrophic if run by accident and should not live in versioned SQL.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "DropdbStmt") {
        ctx.report(node, "noDropDatabase");
    }
}
