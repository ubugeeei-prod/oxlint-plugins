//! Port of `require-index-on-fk-column`: every foreign-key column must have a
//! covering index so that `DELETE` / `UPDATE` of the parent row does not
//! sequentially scan the child table to enforce the constraint.
//!
//! The rule operates on the full statement list (program-exit) so it can
//! correlate `CREATE INDEX` / `ALTER TABLE ADD PK` statements that appear
//! after the `CREATE TABLE` that introduces the FK.

use serde_json::Value;

use crate::ast::{array_field, is_type, str_field};
use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext, loc_of};
use oxlint_plugins_carton::{CompactString, FastHashMap, FastHashSet, SmallVec};

/// A pending FK that still needs a covering index.
struct FkInfo {
    leading_col: CompactString,
    report_loc: DiagnosticLoc,
}

/// Per-table state accumulated while walking all statements.
struct TableState {
    fks: SmallVec<[FkInfo; 4]>,
    indexed_leading_cols: FastHashSet<CompactString>,
}

impl TableState {
    fn new() -> Self {
        Self {
            fks: SmallVec::new(),
            indexed_leading_cols: FastHashSet::default(),
        }
    }
}

/// Extract the string value from a libpg_query String node.
/// Handles `"sval"`, `{"sval": "..."}`, and `{"sval": {"sval": "..."}}`.
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

/// Collect FK and index information from a `CreateStmt`.
fn collect_create_stmt(node: &Value, tables: &mut FastHashMap<CompactString, TableState>) {
    let table_name = node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if table_name.is_empty() {
        return;
    }
    let state = tables
        .entry(CompactString::from(table_name))
        .or_insert_with(TableState::new);

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
                match str_field(constraint, "contype") {
                    Some("CONSTR_FOREIGN") => {
                        if let Some(loc) = loc_of(constraint) {
                            state.fks.push(FkInfo {
                                leading_col: CompactString::from(col_name),
                                report_loc: loc,
                            });
                        }
                    }
                    Some("CONSTR_PRIMARY") | Some("CONSTR_UNIQUE") => {
                        state
                            .indexed_leading_cols
                            .insert(CompactString::from(col_name));
                    }
                    _ => {}
                }
            }
        } else if is_type(elt, "Constraint") {
            match str_field(elt, "contype") {
                Some("CONSTR_FOREIGN") => {
                    let first_attr = array_field(elt, "fk_attrs")
                        .and_then(|a| a.first())
                        .and_then(get_sval);
                    if let (Some(col), Some(loc)) = (first_attr, loc_of(elt)) {
                        state.fks.push(FkInfo {
                            leading_col: CompactString::from(col),
                            report_loc: loc,
                        });
                    }
                }
                Some("CONSTR_PRIMARY") | Some("CONSTR_UNIQUE") => {
                    let first_key = array_field(elt, "keys")
                        .and_then(|k| k.first())
                        .and_then(get_sval);
                    if let Some(col) = first_key {
                        state.indexed_leading_cols.insert(CompactString::from(col));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Collect index information from an `IndexStmt`.
fn collect_index_stmt(node: &Value, tables: &mut FastHashMap<CompactString, TableState>) {
    let table_name = node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if table_name.is_empty() {
        return;
    }
    let first_col = array_field(node, "indexParams")
        .and_then(|p| p.first())
        .and_then(|e| e.get("name"))
        .and_then(Value::as_str);
    if let Some(col) = first_col {
        tables
            .entry(CompactString::from(table_name))
            .or_insert_with(TableState::new)
            .indexed_leading_cols
            .insert(CompactString::from(col));
    }
}

/// Collect FK / index information from an `AlterTableStmt`.
fn collect_alter_table_stmt(node: &Value, tables: &mut FastHashMap<CompactString, TableState>) {
    let table_name = node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if table_name.is_empty() {
        return;
    }
    let cmds = match array_field(node, "cmds") {
        Some(c) => c,
        None => return,
    };
    let state = tables
        .entry(CompactString::from(table_name))
        .or_insert_with(TableState::new);

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
        match str_field(def, "contype") {
            Some("CONSTR_FOREIGN") => {
                let first_attr = array_field(def, "fk_attrs")
                    .and_then(|a| a.first())
                    .and_then(get_sval);
                if let (Some(col), Some(loc)) = (first_attr, loc_of(cmd)) {
                    state.fks.push(FkInfo {
                        leading_col: CompactString::from(col),
                        report_loc: loc,
                    });
                }
            }
            Some("CONSTR_PRIMARY") | Some("CONSTR_UNIQUE") => {
                let first_key = array_field(def, "keys")
                    .and_then(|k| k.first())
                    .and_then(get_sval);
                if let Some(col) = first_key {
                    state.indexed_leading_cols.insert(CompactString::from(col));
                }
            }
            _ => {}
        }
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only execute on the one-time program-level trigger (Value::Null).
    if !node.is_null() {
        return;
    }

    let stmts = ctx.statements;

    // First pass: collect all FK info and indexed columns across all statements.
    let mut tables: FastHashMap<CompactString, TableState> = FastHashMap::default();
    for stmt in stmts {
        match stmt.get("type").and_then(Value::as_str) {
            Some("CreateStmt") => collect_create_stmt(stmt, &mut tables),
            Some("IndexStmt") => collect_index_stmt(stmt, &mut tables),
            Some("AlterTableStmt") => collect_alter_table_stmt(stmt, &mut tables),
            _ => {}
        }
    }

    // Second pass: for each FK whose leading column has no covering index, report.
    // Collect into a SmallVec first to avoid holding a borrow on `tables` while
    // calling ctx.report_loc.
    let mut to_report: SmallVec<[(DiagnosticLoc, CompactString); 4]> = SmallVec::new();
    for state in tables.values() {
        for fk in &state.fks {
            if !state.indexed_leading_cols.contains(&fk.leading_col) {
                to_report.push((fk.report_loc, fk.leading_col.clone()));
            }
        }
    }

    // Sort by loc so diagnostics come out in source order.
    to_report.sort_by_key(|(loc, _)| {
        (
            loc.start_line,
            loc.start_column,
            loc.end_line,
            loc.end_column,
        )
    });

    for (loc, col) in to_report {
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("col"),
            value: col,
        });
        ctx.report_loc(loc, "missingIndex", data, None);
    }
}
