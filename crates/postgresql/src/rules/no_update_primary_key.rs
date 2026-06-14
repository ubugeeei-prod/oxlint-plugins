//! Port of `no-update-primary-key`: disallow `UPDATE ... SET <pk> = ...` for
//! columns the rule treats as primary keys. Default heuristic is any column
//! named `id` plus a per-statement `<table>_id`; the `pkColumnNames` option
//! replaces the default `id` list.

#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "rule helpers operate on arbitrary-length identifier/column lists and reconstructed source text at the rule boundary, where owned String/Vec and per-rule formatting are appropriate"
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "UpdateStmt") {
        return;
    }
    let mut names: Vec<String> = match ctx
        .options
        .get(0)
        .and_then(|o| o.get("pkColumnNames"))
        .and_then(Value::as_array)
    {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        None => vec![String::from("id")],
    };
    if let Some(relname) = field(node, "relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
    {
        names.push(format!("{relname}_id"));
    }
    let Some(targets) = array_field(node, "targetList") else {
        return;
    };
    for target in targets {
        if !is_type(target, "ResTarget") {
            continue;
        }
        let Some(name) = str_field(target, "name") else {
            continue;
        };
        if !names.iter().any(|n| n == name) {
            continue;
        }
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("name"),
            value: CompactString::from(name),
        });
        ctx.report_data(target, "noUpdatePk", data);
    }
}
