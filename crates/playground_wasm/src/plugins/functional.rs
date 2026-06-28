//! Adapter for the `functional` plugin (port of eslint-plugin-functional).

use std::collections::BTreeMap;

use oxlint_plugins_functional as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "functional";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_functional_rule_names()
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
    // The NAPI wrapper, when invoked without per-rule options, reduces to the
    // core defaults (all rules enabled, default flags). The playground has no
    // per-rule UI options, so we reproduce that default construction.
    let core_options = core::FunctionalOptions::default();
    for diagnostic in core::scan_functional(source_text, filename, &core_options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        // index.js maps every messageId to the `{{message}}` template, so the
        // only data placeholder is `message`, fed the rendered diagnostic text.
        let mut data: BTreeMap<&'static str, String> = BTreeMap::new();
        data.insert("message", diagnostic.message.into_string());
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
