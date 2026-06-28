//! Adapter for the `simple-import-sort` plugin (port of eslint-plugin-simple-import-sort).

use std::collections::BTreeMap;

use oxlint_plugins_simple_import_sort as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "simple-import-sort";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_simple_import_sort_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    }
}

pub fn scan(
    source_text: &str,
    filename: &str,
    filter: &EnabledFilter,
    out: &mut Vec<PlaygroundDiagnostic>,
) {
    // The playground has no rule configuration, so we mirror the napi wrapper's
    // `options.unwrap_or_default()` path: None import_groups → default 5 groups.
    let options = core::SimpleImportSortOptions::default();
    for diagnostic in core::scan_simple_import_sort(source_text, filename, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        // The simple-import-sort messages carry no `{{placeholder}}` substitutions,
        // so there is no diagnostic data to forward.
        let data: BTreeMap<String, String> = BTreeMap::new();
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
