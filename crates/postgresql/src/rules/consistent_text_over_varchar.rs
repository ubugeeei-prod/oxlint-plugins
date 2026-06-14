//! Port of `consistent-text-over-varchar`
use crate::RuleContext;
use crate::ast::{get_type_name, is_type};
use serde_json::Value;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    let Some(type_name) = node.get("typeName") else {
        return;
    };
    let Some(t) = get_type_name(type_name) else {
        return;
    };
    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");
    if style == "always" && t == "varchar" {
        // Only flag varchar(n) (with typmods), not plain varchar
        let has_typmods = type_name.get("typmods").and_then(Value::as_array).is_some();
        if has_typmods {
            ctx.report(node, "preferText");
        }
    } else if style == "never" && t == "text" {
        ctx.report(node, "unexpectedText");
    }
}
