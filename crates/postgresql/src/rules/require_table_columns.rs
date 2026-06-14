//! Port of `require-table-columns`: require every `CREATE TABLE` to include a
//! configured set of columns (with optional per-pattern overrides).

#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "rule helpers operate on arbitrary-length identifier/column lists and reconstructed source text at the rule boundary, where owned String/Vec and per-rule formatting are appropriate"
)]

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::ast::{array_field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let option = ctx.options.get(0);
    let base = string_list(option.and_then(|o| o.get("columns")));
    if base.is_empty() {
        return;
    }
    let Some(table_name) = node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
    else {
        return;
    };
    if let Some(ex) = option
        .and_then(|o| o.get("exclude"))
        .and_then(Value::as_str)
        && Regex::new(ex).is_ok_and(|re| re.is_match(table_name))
    {
        return;
    }

    // First matching override wins.
    let mut chosen: Option<(String, Vec<String>)> = None;
    if let Some(overrides) = option
        .and_then(|o| o.get("overrides"))
        .and_then(Value::as_array)
    {
        for o in overrides {
            if let Some(pat) = o.get("pattern").and_then(Value::as_str)
                && Regex::new(pat).is_ok_and(|re| re.is_match(table_name))
            {
                chosen = Some((pat.to_string(), string_list(o.get("columns"))));
                break;
            }
        }
    }

    let (required, rationale) = match &chosen {
        Some((pat, cols)) => (
            cols.clone(),
            format!("Required by the override for pattern `{pat}`."),
        ),
        None => (
            base.clone(),
            "Required by the default column list.".to_string(),
        ),
    };
    if required.is_empty() {
        return;
    }

    let mut present: Vec<&str> = Vec::new();
    if let Some(elts) = array_field(node, "tableElts") {
        for elt in elts {
            if is_type(elt, "ColumnDef")
                && let Some(colname) = str_field(elt, "colname")
            {
                present.push(colname);
            }
        }
    }

    for col in &required {
        if present.contains(&col.as_str()) {
            continue;
        }
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("table"),
            value: CompactString::from(table_name),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("missing"),
            value: CompactString::from(col.as_str()),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("rationale"),
            value: CompactString::from(rationale.as_str()),
        });
        ctx.report_data(node, "missingColumn", data);
    }
}
