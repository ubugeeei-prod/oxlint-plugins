//! Port of `no-update-primary-key`: disallow `UPDATE ... SET <pk> = ...` for
//! columns the rule treats as primary keys.
//!
//! Default heuristic: any column literally named `id`. The option
//! `pkColumnNames` lets a project add its own names. Additionally, for each
//! statement the `<table>_id` column (derived from the relation being updated)
//! is added to the per-statement set.

use serde_json::Value;

use crate::ast::{array_field, is_type, str_field};
use crate::{DiagnosticDatum, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

const DEFAULT_PK_COLUMN_NAMES: &[&str] = &["id"];

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "UpdateStmt") {
        return;
    }

    // Derive the table name from node.relation.relname (if present) to build
    // the <table>_id heuristic on top of the configured global names.
    let relname: Option<&str> = node
        .get("relation")
        .and_then(|r| r.get("relname"))
        .and_then(Value::as_str);

    // Build the set of PK column names: options[0].pkColumnNames ?? DEFAULT, +
    // relname_id if we have a relname. Use SmallVec to stay in the carton
    // allocation policy.
    let configured: SmallVec<[&str; 8]> = ctx
        .options
        .get(0)
        .and_then(|o| o.get("pkColumnNames"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_else(|| DEFAULT_PK_COLUMN_NAMES.iter().copied().collect());

    // Build the <relname>_id derived name using CompactString to avoid String.
    let derived: Option<CompactString> = relname.map(|r| {
        let mut cs = CompactString::from(r);
        cs.push_str("_id");
        cs
    });

    let target_list = match array_field(node, "targetList") {
        Some(list) => list,
        None => return,
    };

    for target in target_list {
        if !is_type(target, "ResTarget") {
            continue;
        }
        let name = match str_field(target, "name") {
            Some(n) => n,
            None => continue,
        };

        let is_pk = configured.contains(&name) || derived.as_deref().is_some_and(|d| d == name);

        if !is_pk {
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
