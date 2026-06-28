//! Adapter for the `testing-library` plugin (port of eslint-plugin-testing-library).

use std::collections::BTreeMap;

use oxlint_plugins_testing_library as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "testing-library";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_testing_library_rule_names()
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
    // Mirror the NAPI wrapper with no per-call options: the default options
    // enable every implemented rule with upstream `consistent-data-testid`
    // defaults (empty pattern, `data-testid` attribute, no custom message).
    let options = core::TestingLibraryOptions::default();
    for diagnostic in core::scan_testing_library(source_text, filename, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        // The core diagnostic carries an already-rendered message; the npm
        // plugin reports it via message id `unexpected` with template
        // `{{message}}`, so reproduce that here.
        let mut data: BTreeMap<String, String> = BTreeMap::new();
        data.insert("message".to_owned(), diagnostic.message.to_string());
        out.push(PlaygroundDiagnostic {
            plugin: PLUGIN,
            rule: diagnostic.rule_name.to_owned(),
            message_id: "unexpected".to_owned(),
            data,
            start_line: diagnostic.loc.start_line,
            start_column: diagnostic.loc.start_column,
            end_line: diagnostic.loc.end_line,
            end_column: diagnostic.loc.end_column,
        });
    }
}
