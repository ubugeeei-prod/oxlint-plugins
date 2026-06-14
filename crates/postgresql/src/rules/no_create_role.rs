//! Port of `no-create-role`: disallow `CREATE ROLE` / `CREATE USER` in
//! application migrations; manage roles in a separate operator workflow.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "CreateRoleStmt") {
        ctx.report(node, "noCreateRole");
    }
}
