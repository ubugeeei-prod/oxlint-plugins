//! Adapter for the `storybook` plugin (port of eslint-plugin-storybook).

use std::collections::BTreeMap;

use oxlint_plugins_carton::CompactString;
use oxlint_plugins_storybook as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "storybook";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_storybook_rule_names()
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
    let options = core::StorybookOptions::default();
    for diagnostic in core::scan_storybook(source_text, filename, &options) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<&'static str, String> = BTreeMap::new();
        let d = diagnostic.data;
        push(&mut data, "method", d.method);
        push(&mut data, "metaTitle", d.meta_title);
        push(&mut data, "property", d.property);
        push(&mut data, "rendererPackage", d.renderer_package);
        push(&mut data, "suggestions", d.suggestions);
        push(&mut data, "library", d.library);
        push(&mut data, "addonName", d.addon_name);
        push(&mut data, "packageJsonPath", d.package_json_path);
        push(&mut data, "name", d.name);
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
