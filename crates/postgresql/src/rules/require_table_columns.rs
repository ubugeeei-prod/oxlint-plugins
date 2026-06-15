//! Port of `require-table-columns`: every `CREATE TABLE` must contain a
//! required set of columns, with per-table-name pattern overrides and an
//! optional exclusion pattern.

use regex::Regex;
use serde_json::Value;

use crate::ast::{array_field, is_type};
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }

    // Copy the options reference before any ctx mutation to avoid borrow conflict.
    let options = ctx.options;

    let opts = match options.get(0) {
        Some(o) => o,
        None => return,
    };

    let base_columns: SmallVec<[&str; 8]> = opts
        .get("columns")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    if base_columns.is_empty() {
        return;
    }

    let table_name = node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
        .unwrap_or("");

    // Check exclude pattern — skip this table entirely if it matches.
    if let Some(exclude) = opts.get("exclude").and_then(Value::as_str)
        && Regex::new(exclude).is_ok_and(|re| re.is_match(table_name))
    {
        return;
    }

    // Find the first matching override (first-match-wins).
    let (required_columns, rationale): (SmallVec<[&str; 8]>, CompactString) =
        if let Some(overrides) = opts.get("overrides").and_then(Value::as_array) {
            let mut found: Option<(SmallVec<[&str; 8]>, CompactString)> = None;
            for ov in overrides {
                let pattern = match ov.get("pattern").and_then(Value::as_str) {
                    Some(p) => p,
                    None => continue,
                };
                if Regex::new(pattern).is_ok_and(|re| re.is_match(table_name)) {
                    let cols: SmallVec<[&str; 8]> = ov
                        .get("columns")
                        .and_then(Value::as_array)
                        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                        .unwrap_or_default();
                    let mut r = CompactString::new("");
                    r.push_str("Required by the override for pattern `");
                    r.push_str(pattern);
                    r.push_str("`.");
                    found = Some((cols, r));
                    break;
                }
            }
            found.unwrap_or_else(|| {
                (
                    base_columns.clone(),
                    CompactString::from("Required by the default column list."),
                )
            })
        } else {
            (
                base_columns,
                CompactString::from("Required by the default column list."),
            )
        };

    // Collect the column names that are actually present in the table definition.
    let present: SmallVec<[&str; 8]> = array_field(node, "tableElts")
        .unwrap_or(&[])
        .iter()
        .filter(|elt| is_type(elt, "ColumnDef"))
        .filter_map(|elt| elt.get("colname").and_then(Value::as_str))
        .collect();

    // Report each required column that is absent.
    let table_cs = CompactString::from(table_name);
    for col in &required_columns {
        if !present.contains(col) {
            let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
            data.push(DiagnosticDatum {
                key: CompactString::from("table"),
                value: table_cs.clone(),
            });
            data.push(DiagnosticDatum {
                key: CompactString::from("missing"),
                value: CompactString::from(*col),
            });
            data.push(DiagnosticDatum {
                key: CompactString::from("rationale"),
                value: rationale.clone(),
            });
            ctx.report_data(node, "missingColumn", data);
        }
    }
}
