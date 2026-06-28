//! Adapter for the `e18e` plugin (port of @e18e/eslint-plugin).

use std::collections::BTreeMap;

use oxlint_plugins_carton::CompactString;
use oxlint_plugins_e18e as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "e18e";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_e18e_rule_names()
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
    let options = scan_options();
    for diagnostic in core::scan_e18e(source_text, filename, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<&'static str, String> = BTreeMap::new();
        let d = diagnostic.data;
        push(&mut data, "array", d.array);
        push(&mut data, "index", d.index);
        push(&mut data, "item", d.item);
        push(&mut data, "length", d.length);
        push(&mut data, "value", d.value);
        push(&mut data, "iterable", d.iterable);
        push(&mut data, "mapper", d.mapper);
        push(&mut data, "regex", d.regex);
        push(&mut data, "string", d.string);
        push(&mut data, "original", d.original);
        push(&mut data, "name", d.name);
        push(&mut data, "replacement", d.replacement);
        push(&mut data, "url", d.url);
        push(&mut data, "description", d.description);
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

/// Builds the scan options the playground runs with: every rule enabled and the
/// default banned-dependencies preset, mirroring `npm/e18e/index.js`.
fn scan_options() -> core::E18eOptions {
    core::E18eOptions {
        banned_dependencies: default_banned_dependencies(),
        ..core::E18eOptions::default()
    }
}

fn default_banned_dependencies() -> oxlint_plugins_carton::SmallVec<[core::BanDependency; 16]> {
    let mut banned = oxlint_plugins_carton::SmallVec::new();
    banned.push(core::BanDependency {
        module_name: CompactString::from("lodash.merge"),
        message_id: CompactString::from("documentedReplacement"),
        replacement: Some(CompactString::from("deepmerge-ts")),
        url: Some(CompactString::from(
            "https://github.com/es-tooling/module-replacements",
        )),
        description: None,
    });
    banned.push(core::BanDependency {
        module_name: CompactString::from("lodash.clonedeep"),
        message_id: CompactString::from("documentedReplacement"),
        replacement: Some(CompactString::from("structuredClone")),
        url: Some(CompactString::from(
            "https://github.com/es-tooling/module-replacements",
        )),
        description: None,
    });
    banned.push(core::BanDependency {
        module_name: CompactString::from("left-pad"),
        message_id: CompactString::from("removalReplacement"),
        replacement: None,
        url: None,
        description: Some(CompactString::from(
            "This module is no longer needed in modern JavaScript.",
        )),
    });
    banned
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
