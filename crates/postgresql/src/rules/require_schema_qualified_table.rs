//! Port of `require-schema-qualified-table`: every `CREATE TABLE` should
//! specify an explicit schema to avoid depending on `search_path`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let schemaname = node
        .get("relation")
        .and_then(|r| r.get("schemaname"))
        .and_then(Value::as_str);
    match schemaname {
        Some(s) if !s.is_empty() => {} // qualified — OK
        _ => ctx.report(node, "requireSchemaQualifiedTable"),
    }
}
