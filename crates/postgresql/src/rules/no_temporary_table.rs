//! Port of `no-temporary-table`: disallow `CREATE TEMPORARY TABLE` in versioned
//! SQL — temp tables exist for the session only and rarely belong in migration
//! files.

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
    if str_field(relation, "relpersistence") == Some("t") {
        ctx.report(node, "noTemporaryTable");
    }
}
