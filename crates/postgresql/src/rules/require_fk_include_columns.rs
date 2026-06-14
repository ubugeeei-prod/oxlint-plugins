//! Port of `require-fk-include-columns`: every foreign-key constraint must
//! include a configurable set of columns (e.g. a tenant key) so that child
//! rows cannot reference a parent in a different tenant.

use regex::Regex;
use serde_json::Value;

use crate::ast::{array_field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Extract the string value from a libpg_query String node.
fn get_sval(n: &Value) -> Option<&str> {
    if let Some(s) = n.as_str() {
        return Some(s);
    }
    if let Some(sval) = n.get("sval") {
        if let Some(s) = sval.as_str() {
            return Some(s);
        }
        if let Some(s) = sval.get("sval").and_then(Value::as_str) {
            return Some(s);
        }
    }
    None
}

/// Collect the FK referencing column names from a `fk_attrs` array.
fn collect_fk_attrs(attrs: &[Value]) -> SmallVec<[&str; 4]> {
    attrs.iter().filter_map(get_sval).collect()
}

struct FkCandidate<'a> {
    report_node: &'a Value,
    fk_columns: SmallVec<[&'a str; 4]>,
    /// `None` when `relation.relname` is absent; mirrors upstream `tableName`
    /// (which is `undefined` / falsy when absent) for the short-circuit guard.
    table_name: Option<&'a str>,
    ref_table: &'a str,
}

/// Check a candidate FK and report any missing required columns.
fn check<'a>(
    candidate: FkCandidate<'a>,
    required_columns: &[&str],
    exclude_table_pattern: Option<&str>,
    exclude_ref_table_pattern: Option<&str>,
    ctx: &mut RuleContext,
) {
    // Mirror upstream `tableName && excludeTablePattern.test(tableName)`:
    // only run the regex when table_name is present (non-None / non-empty).
    if let (Some(pat), Some(name)) = (exclude_table_pattern, candidate.table_name)
        && !name.is_empty()
        && Regex::new(pat).is_ok_and(|re| re.is_match(name))
    {
        return;
    }
    // Skip if the referenced table matches the exclusion pattern.
    if let Some(pat) = exclude_ref_table_pattern
        && Regex::new(pat).is_ok_and(|re| re.is_match(candidate.ref_table))
    {
        return;
    }

    // When table name is absent, display "(unknown)" in the message, matching
    // upstream's `tableName || "(unknown)"` fallback.
    let table_cs = CompactString::from(candidate.table_name.unwrap_or("(unknown)"));
    let ref_cs = CompactString::from(candidate.ref_table);

    for &required in required_columns {
        if !candidate.fk_columns.contains(&required) {
            let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
            data.push(DiagnosticDatum {
                key: CompactString::from("table"),
                value: table_cs.clone(),
            });
            data.push(DiagnosticDatum {
                key: CompactString::from("refTable"),
                value: ref_cs.clone(),
            });
            data.push(DiagnosticDatum {
                key: CompactString::from("missing"),
                value: CompactString::from(required),
            });
            ctx.report_data(candidate.report_node, "missingFkColumn", data);
        }
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Copy option references before any ctx mutation.
    let options = ctx.options;

    let opts = match options.get(0) {
        Some(o) => o,
        None => return,
    };

    let required_columns: SmallVec<[&str; 4]> = opts
        .get("columns")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    if required_columns.is_empty() {
        return;
    }

    let exclude_table_pattern = opts.get("excludeTablePattern").and_then(Value::as_str);
    let exclude_ref_table_pattern = opts
        .get("excludeReferencedTablePattern")
        .and_then(Value::as_str);

    if is_type(node, "CreateStmt") {
        let table_name = node
            .get("relation")
            .and_then(|r| r.get("relname"))
            .and_then(Value::as_str);

        let elts = match array_field(node, "tableElts") {
            Some(e) => e,
            None => return,
        };

        for elt in elts {
            if is_type(elt, "ColumnDef") {
                let col_name = match elt.get("colname").and_then(Value::as_str) {
                    Some(n) => n,
                    None => continue,
                };
                let constraints = match array_field(elt, "constraints") {
                    Some(c) => c,
                    None => continue,
                };
                for constraint in constraints {
                    if !is_type(constraint, "Constraint") {
                        continue;
                    }
                    if str_field(constraint, "contype") != Some("CONSTR_FOREIGN") {
                        continue;
                    }
                    let ref_table = constraint
                        .get("pktable")
                        .and_then(|t| t.get("relname"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    // For an inline column FK the referencing columns list is
                    // just [colname] — there is no fk_attrs in this case.
                    let mut fk_cols: SmallVec<[&str; 4]> = SmallVec::new();
                    fk_cols.push(col_name);
                    check(
                        FkCandidate {
                            report_node: constraint,
                            fk_columns: fk_cols,
                            table_name,
                            ref_table,
                        },
                        &required_columns,
                        exclude_table_pattern,
                        exclude_ref_table_pattern,
                        ctx,
                    );
                }
            } else if is_type(elt, "Constraint") {
                if str_field(elt, "contype") != Some("CONSTR_FOREIGN") {
                    continue;
                }
                let ref_table = elt
                    .get("pktable")
                    .and_then(|t| t.get("relname"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let fk_cols = array_field(elt, "fk_attrs")
                    .map(collect_fk_attrs)
                    .unwrap_or_default();
                check(
                    FkCandidate {
                        report_node: elt,
                        fk_columns: fk_cols,
                        table_name,
                        ref_table,
                    },
                    &required_columns,
                    exclude_table_pattern,
                    exclude_ref_table_pattern,
                    ctx,
                );
            }
        }
        return;
    }

    if is_type(node, "AlterTableStmt") {
        let table_name = node
            .get("relation")
            .and_then(|r| r.get("relname"))
            .and_then(Value::as_str);

        let cmds = match array_field(node, "cmds") {
            Some(c) => c,
            None => return,
        };

        for cmd in cmds {
            if !is_type(cmd, "AlterTableCmd") {
                continue;
            }
            if str_field(cmd, "subtype") != Some("AT_AddConstraint") {
                continue;
            }
            let def = match cmd.get("def") {
                Some(d) if is_type(d, "Constraint") => d,
                _ => continue,
            };
            if str_field(def, "contype") != Some("CONSTR_FOREIGN") {
                continue;
            }
            let ref_table = def
                .get("pktable")
                .and_then(|t| t.get("relname"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let fk_cols = array_field(def, "fk_attrs")
                .map(collect_fk_attrs)
                .unwrap_or_default();
            check(
                FkCandidate {
                    report_node: cmd,
                    fk_columns: fk_cols,
                    table_name,
                    ref_table,
                },
                &required_columns,
                exclude_table_pattern,
                exclude_ref_table_pattern,
                ctx,
            );
        }
    }
}
