//! Port of `consistent-timestamptz`: enforce a consistent stance on `timestamptz`
//! vs `timestamp` (without time zone). `TIMESTAMP WITH TIME ZONE` parses as
//! `timestamptz`, so it is treated identically.

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
        if t == Some("timestamp") {
            ctx.report(node, "preferTimestamptz");
        }
        return;
    }
    if t == Some("timestamptz") {
        ctx.report(node, "unexpectedTimestamptz");
    }
}
