//! Port of `consistent-drop-index-concurrently`: enforce a consistent stance
//! on `CONCURRENTLY` for `DROP INDEX` (either always require it, or always
//! forbid it). Non-index `DROP`s are out of scope.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

fn style(options: &Value) -> &str {
    options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always")
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "DropStmt") {
        return;
    }
    if str_field(node, "removeType") != Some("OBJECT_INDEX") {
        return;
    }
    let is_concurrent = node.get("concurrent") == Some(&Value::Bool(true));
    let opt = style(ctx.options);
    let always = opt == "always";
    let never = opt == "never";
    if always && !is_concurrent {
        ctx.report(node, "preferConcurrently");
    } else if never && is_concurrent {
        ctx.report(node, "unexpectedConcurrently");
    }
}
