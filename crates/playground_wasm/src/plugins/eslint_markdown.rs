//! Adapter for the `markdown` plugin (port of @eslint/markdown).

use std::collections::BTreeMap;

use oxlint_plugins_carton::CompactString;
use oxlint_plugins_eslint_markdown as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "markdown";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_eslint_markdown_rule_names()
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
    // The NAPI wrapper builds `ScanOptions` from defaults; when no rule names are
    // supplied it enables every implemented rule. Reproduce that here so the
    // playground runs the full rule set, matching the published plugin.
    let options = core::ScanOptions {
        rule_names: core::implemented_eslint_markdown_rule_names()
            .iter()
            .map(|name| CompactString::from(*name))
            .collect(),
        ..core::ScanOptions::default()
    };

    for diagnostic in core::scan_eslint_markdown(source_text, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<&'static str, String> = BTreeMap::new();
        let d = diagnostic.data;
        push(&mut data, "lang", d.lang);
        push(&mut data, "name", d.name);
        push(&mut data, "identifier", d.identifier);
        push(&mut data, "label", d.label);
        if let Some(value) = d.first_line {
            data.insert("firstLine", value.to_string());
        }
        push(&mut data, "firstLabel", d.first_label);
        if let Some(value) = d.from_level {
            data.insert("fromLevel", value.to_string());
        }
        if let Some(value) = d.to_level {
            data.insert("toLevel", value.to_string());
        }
        push(&mut data, "position", d.position);
        push(&mut data, "text", d.text);
        // The native diagnostic carries the node kind as `linkType`; upstream
        // reports it under the `type` placeholder (see index.js `dataForReport`).
        push(&mut data, "type", d.link_type);
        push(&mut data, "prefix", d.prefix);
        push(&mut data, "fragment", d.fragment);
        if let Some(value) = d.expected_cells {
            data.insert("expectedCells", value.to_string());
        }
        if let Some(value) = d.actual_cells {
            data.insert("actualCells", value.to_string());
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

fn push(
    data: &mut BTreeMap<&'static str, String>,
    key: &'static str,
    value: Option<CompactString>,
) {
    if let Some(value) = value {
        data.insert(key, value.as_str().to_owned());
    }
}
