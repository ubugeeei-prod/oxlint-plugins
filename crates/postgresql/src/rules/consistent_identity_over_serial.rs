//! Port of `consistent-identity-over-serial`: enforce a consistent stance on
//! `GENERATED ... AS IDENTITY` vs the `serial` / `bigserial` / `smallserial`
//! pseudo-types on column definitions.

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field, type_name};
use crate::{DiagnosticDatum, RuleContext};

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

fn has_identity(node: &Value) -> bool {
    array_field(node, "constraints").is_some_and(|cs| {
        cs.iter()
            .any(|c| is_type(c, "Constraint") && str_field(c, "contype") == Some("CONSTR_IDENTITY"))
    })
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ColumnDef") {
        return;
    }
    if style(ctx) == "always" {
        if let Some(t) = type_name(field(node, "typeName"))
            && matches!(t, "smallserial" | "serial" | "bigserial")
        {
            let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
            data.push(DiagnosticDatum {
                key: CompactString::from("type"),
                value: CompactString::from(t),
            });
            ctx.report_data(node, "preferIdentity", data);
        }
        return;
    }
    if has_identity(node) {
        ctx.report(node, "unexpectedIdentity");
    }
}
