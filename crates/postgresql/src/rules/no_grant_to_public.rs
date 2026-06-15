//! Port of `no-grant-to-public`: disallow GRANT statements that target the
//! `PUBLIC` pseudo-role. PUBLIC covers every current and future role in the
//! database; naming the specific role(s) is safer.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "GrantStmt") {
        return;
    }
    let is_grant = node
        .get("is_grant")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !is_grant {
        return;
    }
    let Some(grantees) = array_field(node, "grantees") else {
        return;
    };
    for g in grantees {
        if str_field(g, "roletype") == Some("ROLESPEC_PUBLIC") {
            ctx.report(node, "noPublic");
            return;
        }
    }
}
