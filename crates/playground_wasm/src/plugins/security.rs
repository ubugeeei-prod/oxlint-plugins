//! Adapter for the `security` plugin (port of eslint-plugin-security).

use std::collections::BTreeMap;

use oxlint_plugins_security as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "security";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_security_rule_names()
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
    for diagnostic in core::scan_security(source_text, filename) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<String, String> = BTreeMap::new();
        let d = diagnostic.data;
        super::push(&mut data, "text", d.text);
        super::push(&mut data, "method", d.method);
        super::push(&mut data, "packageName", d.package_name);
        super::push(&mut data, "fnName", d.fn_name);
        super::push(&mut data, "indices", d.indices);
        super::push(&mut data, "side", d.side);
        super::push(&mut data, "value", d.value);
        super::push(&mut data, "argumentType", d.argument_type);
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
