//! Adapter for the `regexp` plugin (port of eslint-plugin-regexp).

use std::collections::BTreeMap;

use oxlint_plugins_regexp as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "regexp";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_regexp_rule_names()
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
    for diagnostic in core::scan_regexp(source_text, filename) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<String, String> = BTreeMap::new();
        let d = diagnostic.data;
        super::push(&mut data, "message", d.message);
        super::push(&mut data, "flag", d.flag);
        super::push(&mut data, "flags", d.flags);
        super::push(&mut data, "sortedFlags", d.sorted_flags);
        super::push(&mut data, "expr", d.expr);
        // `index.js` `compactData` mirrors `charText` into the `char` key used by
        // the message placeholders (`{{ char }}`); reproduce that here.
        super::push(&mut data, "charText", d.char_text.clone());
        super::push(&mut data, "char", d.char_text);
        super::push(&mut data, "replacement", d.replacement);
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
