//! Adapter for the `unused-imports` plugin (port of eslint-plugin-unused-imports).

use std::collections::BTreeMap;

use oxlint_plugins_unused_imports as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "unused-imports";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_unused_imports_rule_names()
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
    let options = core::UnusedImportsOptions::default();
    for diagnostic in core::scan_unused_imports(source_text, filename, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<&'static str, String> = BTreeMap::new();
        data.insert("message", diagnostic.message.as_str().to_owned());
        out.push(PlaygroundDiagnostic {
            plugin: PLUGIN,
            rule: diagnostic.rule_name.to_owned(),
            message_id: "unused".to_owned(),
            data,
            start_line: diagnostic.loc.start_line,
            start_column: diagnostic.loc.start_column,
            end_line: diagnostic.loc.end_line,
            end_column: diagnostic.loc.end_column,
        });
    }
}
