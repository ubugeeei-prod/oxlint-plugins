//! Port of `consistent-jsonb-over-json`: enforce a consistent stance on `jsonb`
//! vs `json` for column types.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, type_name};

fn style(ctx: &RuleContext) -> &'static str {
    match ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
    {
        Some("never") => "never",
        _ => "always",
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    let t = type_name(field(node, "typeName"));
    if style(ctx) == "always" {
        if t == Some("json") {
            ctx.report(node, "preferJsonb");
        }
        return;
    }
    if t == Some("jsonb") {
        ctx.report(node, "unexpectedJsonb");
    }
}
