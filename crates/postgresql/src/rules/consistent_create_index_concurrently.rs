//! Port of `consistent-create-index-concurrently`
use crate::RuleContext;
use crate::ast::is_type;
use serde_json::Value;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "IndexStmt") {
        return;
    }
    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");
    let concurrent = node
        .get("concurrent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if style == "always" && !concurrent {
        ctx.report(node, "preferConcurrently");
    } else if style == "never" && concurrent {
        ctx.report(node, "unexpectedConcurrently");
    }
}
