//! Port of `require-fk-include-columns`: require every foreign-key constraint
//! to include a configured set of columns (e.g. `tenant_id`).

#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "rule helpers operate on arbitrary-length identifier/column lists and reconstructed source text at the rule boundary, where owned String/Vec and per-rule formatting are appropriate"
)]

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::ast::{array_field, field, is_type, node_type, str_field};
use crate::{DiagnosticDatum, RuleContext};

fn collect_fk_attrs(fk_attrs: Option<&[Value]>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = fk_attrs {
        for a in arr {
            if let Some(s) = a.get("sval").and_then(Value::as_str) {
                out.push(s.to_string());
            }
        }
    }
    out
}

fn referenced_table_name(constraint: &Value) -> Option<String> {
    constraint
        .get("pktable")
        .and_then(|p| p.get("relname"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[allow(clippy::too_many_arguments)]
fn check(
    ctx: &mut RuleContext,
    report_node: &Value,
    table_name: Option<&str>,
    constraint: &Value,
    fk_columns: &[String],
    required: &[String],
    table_exclude: &Option<Regex>,
    ref_exclude: &Option<Regex>,
) {
    if let (Some(t), Some(re)) = (table_name, table_exclude)
        && re.is_match(t)
    {
        return;
    }
    let ref_table = referenced_table_name(constraint);
    if let (Some(rt), Some(re)) = (ref_table.as_deref(), ref_exclude)
        && re.is_match(rt)
    {
        return;
    }
    for col in required {
        if fk_columns.iter().any(|c| c == col) {
            continue;
        }
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("table"),
            value: CompactString::from(table_name.unwrap_or("(unknown)")),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("refTable"),
            value: CompactString::from(ref_table.as_deref().unwrap_or("(unknown)")),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("missing"),
            value: CompactString::from(col.as_str()),
        });
        ctx.report_data(report_node, "missingFkColumn", data);
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    let option = ctx.options.get(0);
    let required: Vec<String> = option
        .and_then(|o| o.get("columns"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if required.is_empty() {
        return;
    }
    let table_exclude = option
        .and_then(|o| o.get("excludeTablePattern"))
        .and_then(Value::as_str)
        .and_then(|p| Regex::new(p).ok());
    let ref_exclude = option
        .and_then(|o| o.get("excludeReferencedTablePattern"))
        .and_then(Value::as_str)
        .and_then(|p| Regex::new(p).ok());

    if is_type(node, "CreateStmt") {
        let table_name = node
            .get("relation")
            .and_then(|r| r.get("relname"))
            .and_then(Value::as_str);
        let Some(elts) = array_field(node, "tableElts") else {
            return;
        };
        for elt in elts {
            match node_type(elt) {
                Some("ColumnDef") => {
                    if let Some(colname) = str_field(elt, "colname")
                        && let Some(cons) = array_field(elt, "constraints")
                    {
                        for c in cons {
                            if is_type(c, "Constraint")
                                && str_field(c, "contype") == Some("CONSTR_FOREIGN")
                            {
                                check(
                                    ctx,
                                    c,
                                    table_name,
                                    c,
                                    &[colname.to_string()],
                                    &required,
                                    &table_exclude,
                                    &ref_exclude,
                                );
                            }
                        }
                    }
                }
                Some("Constraint") if str_field(elt, "contype") == Some("CONSTR_FOREIGN") => {
                    let cols = collect_fk_attrs(array_field(elt, "fk_attrs"));
                    check(
                        ctx,
                        elt,
                        table_name,
                        elt,
                        &cols,
                        &required,
                        &table_exclude,
                        &ref_exclude,
                    );
                }
                _ => {}
            }
        }
    } else if is_type(node, "AlterTableStmt") {
        let table_name = node
            .get("relation")
            .and_then(|r| r.get("relname"))
            .and_then(Value::as_str);
        let Some(cmds) = array_field(node, "cmds") else {
            return;
        };
        for cmd in cmds {
            if str_field(cmd, "subtype") != Some("AT_AddConstraint") {
                continue;
            }
            let Some(def) = field(cmd, "def") else {
                continue;
            };
            if !is_type(def, "Constraint") || str_field(def, "contype") != Some("CONSTR_FOREIGN") {
                continue;
            }
            let cols = collect_fk_attrs(array_field(def, "fk_attrs"));
            check(
                ctx,
                cmd,
                table_name,
                def,
                &cols,
                &required,
                &table_exclude,
                &ref_exclude,
            );
        }
    }
}
