//! Adapter for the `postgresql` plugin (port of eslint-plugin-postgresql).
//!
//! libpg_query's C parser has no `wasm32` build, so unlike the other plugins
//! this one cannot parse in the browser. The frontend parses the SQL with
//! `@libpg-query/parser` (the same libpg_query, compiled to WASM via Emscripten)
//! and passes the resulting JSON parse tree here.

use std::collections::BTreeMap;

use oxlint_plugins_carton::CompactString;
use oxlint_plugins_postgresql as core;
use serde_json::Value;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "postgresql";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_postgresql_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    }
}

/// Lints `source_text` using `raw_json`, libpg_query's JSON parse tree for it.
pub fn scan(
    source_text: &str,
    raw_json: &str,
    filter: &EnabledFilter,
    out: &mut Vec<PlaygroundDiagnostic>,
) {
    let rule_names = core::implemented_postgresql_rule_names()
        .iter()
        .filter(|name| filter.rule_enabled(PLUGIN, name))
        .map(|name| CompactString::from(*name))
        .collect();
    // No options UI: every rule reads its own defaults from a null options value.
    let options = core::ScanOptions {
        rule_names,
        options: Value::Null,
    };

    for diagnostic in core::scan_postgresql_from_raw(source_text, raw_json, &options) {
        let mut data: BTreeMap<String, String> = BTreeMap::new();
        for datum in diagnostic.data {
            data.insert(
                datum.key.as_str().to_owned(),
                datum.value.as_str().to_owned(),
            );
        }
        out.push(PlaygroundDiagnostic {
            plugin: PLUGIN,
            rule: diagnostic.rule_name.to_owned(),
            message_id: diagnostic.message_id.to_owned(),
            data,
            start_line: diagnostic.loc.start_line,
            start_column: diagnostic.loc.start_column,
            end_line: diagnostic.loc.end_line,
            end_column: diagnostic.loc.end_column,
        });
    }
}
