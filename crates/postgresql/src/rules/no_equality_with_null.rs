//! Port of `no-equality-with-null`: disallow `x = NULL` / `x <> NULL`.
//! PostgreSQL's three-valued logic makes both evaluate to NULL (never true),
//! silently filtering rows the author probably wanted.

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};

fn is_null_const(node: &Value) -> bool {
    is_type(node, "A_Const") && node.get("isnull") == Some(&Value::Bool(true))
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "A_Expr") {
        return;
    }
    if str_field(node, "kind") != Some("AEXPR_OP") {
        return;
    }
    let Some(name) = array_field(node, "name") else {
        return;
    };
    if name.len() != 1 {
        return;
    }
    let Some(op) = name[0].get("sval").and_then(Value::as_str) else {
        return;
    };
    if op != "=" && op != "<>" {
        return;
    }
    let lexpr_null = field(node, "lexpr").is_some_and(is_null_const);
    let rexpr_null = field(node, "rexpr").is_some_and(is_null_const);
    if !lexpr_null && !rexpr_null {
        return;
    }
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("op"),
        value: CompactString::from(op),
    });
    ctx.report_data(node, "useIsNull", data);
}
