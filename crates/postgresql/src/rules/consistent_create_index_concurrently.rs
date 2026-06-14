//! Port of `consistent-create-index-concurrently`: enforce a consistent
//! stance on `CONCURRENTLY` for `CREATE INDEX` (either always require it, or
//! always forbid it).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

fn style(options: &Value) -> &str {
    options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always")
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "IndexStmt") {
        return;
    }
    let has_concurrently = node.get("concurrent") == Some(&Value::Bool(true));
    let opt = style(ctx.options);
    let always = opt == "always";
    let never = opt == "never";
    if always && !has_concurrently {
        ctx.report(node, "preferConcurrently");
    } else if never && has_concurrently {
        ctx.report(node, "unexpectedConcurrently");
    }
}
