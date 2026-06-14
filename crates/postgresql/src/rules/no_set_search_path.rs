//! Port of `no-set-search-path`: disallow `SET search_path = ...` in versioned
//! SQL. Changing `search_path` makes name resolution depend on session state
//! and is a known security foot-gun, especially inside `SECURITY DEFINER`
//! functions. Identifiers should be schema-qualified instead.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "VariableSetStmt") {
        return;
    }
    if str_field(node, "name") == Some("search_path") {
        ctx.report(node, "noSetSearchPath");
    }
}
