//! Port of `require-schema-qualified-table`: require `CREATE TABLE` to use a
//! schema-qualified name (e.g. `audit.events`).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let has_schema = node
        .get("relation")
        .and_then(|r| r.get("schemaname"))
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty());
    if has_schema {
        return;
    }
    ctx.report(node, "requireSchemaQualifiedTable");
}
