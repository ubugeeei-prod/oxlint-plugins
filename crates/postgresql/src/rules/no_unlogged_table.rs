//! Port of `no-unlogged-table`: disallow `CREATE UNLOGGED TABLE` because
//! unlogged tables are truncated on crash and not replicated.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let Some(relation) = field(node, "relation") else {
        return;
    };
    if str_field(relation, "relpersistence") == Some("u") {
        ctx.report(node, "noUnloggedTable");
    }
}
