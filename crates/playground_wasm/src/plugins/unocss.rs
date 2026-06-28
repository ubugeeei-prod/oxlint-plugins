//! Adapter for the `@unocss` plugin (port of @unocss/eslint-plugin).

use std::collections::BTreeMap;

use oxlint_plugins_unocss as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "@unocss";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_unocss_rule_names()
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
    // The npm wrapper builds its core options from optional UI input; with no
    // playground-supplied options every field collapses to the crate defaults.
    let options = core::UnocssOptions::default();
    for diagnostic in core::scan_unocss(source_text, filename, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<String, String> = BTreeMap::new();
        super::push(&mut data, "name", diagnostic.name);
        super::push(&mut data, "reason", diagnostic.reason);
        super::push(&mut data, "prefix", diagnostic.prefix);
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
