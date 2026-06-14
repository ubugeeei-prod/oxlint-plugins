//! Port of `consistent-text-over-varchar`: enforce a consistent stance on `text`
//! vs `varchar(n)` for string columns. The `always` style only flags
//! length-bounded `varchar(n)` (i.e. typeName carries `typmods`), never the
//! unbounded `varchar`.

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
    let type_name_node = field(node, "typeName");
    let name = type_name(type_name_node);
    if style(ctx) == "always" {
        let has_typmods = type_name_node
            .and_then(|t| t.get("typmods"))
            .and_then(Value::as_array)
            .is_some();
        if name == Some("varchar") && has_typmods {
            ctx.report(node, "preferText");
        }
        return;
    }
    if name == Some("text") {
        ctx.report(node, "unexpectedText");
    }
}
