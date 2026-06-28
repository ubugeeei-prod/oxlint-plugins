//! Adapter for the `json` plugin (port of @eslint/json).

use std::collections::BTreeMap;

use oxlint_plugins_carton::CompactString;
use oxlint_plugins_eslint_json as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "json";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_eslint_json_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    }
}

pub fn scan(
    source_text: &str,
    _filename: &str,
    filter: &EnabledFilter,
    out: &mut Vec<PlaygroundDiagnostic>,
) {
    // The NAPI wrapper passes per-rule options, but with no options it falls
    // back to defaults, which leaves every rule enabled and uses NFC / ascending
    // / case-sensitive / min-keys 2 / non-natural sort. The playground enables
    // all rules and applies rule selection through `filter`, so the defaults
    // reproduce the published plugin's behavior faithfully.
    let options = core::ScanOptions::default();
    for diagnostic in core::scan_eslint_json(source_text, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<&'static str, String> = BTreeMap::new();
        let d = diagnostic.data;
        push(&mut data, "key", d.key);
        push(&mut data, "value", d.value);
        push(&mut data, "surrogate", d.surrogate);
        push(&mut data, "type", d.type_name);
        push(&mut data, "thisName", d.this_name);
        push(&mut data, "prevName", d.prev_name);
        push(&mut data, "direction", d.direction);
        push(&mut data, "sensitivity", d.sensitivity);
        push(&mut data, "sortName", d.sort_name);
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

fn push(
    data: &mut BTreeMap<&'static str, String>,
    key: &'static str,
    value: Option<CompactString>,
) {
    if let Some(value) = value {
        data.insert(key, value.as_str().to_owned());
    }
}
