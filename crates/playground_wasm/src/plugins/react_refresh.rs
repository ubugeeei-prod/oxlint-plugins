//! Adapter for the `react-refresh` plugin (port of eslint-plugin-react-refresh).

use std::collections::BTreeMap;

use oxlint_plugins_react_refresh as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "react-refresh";

/// The plugin exposes a single rule (`RULE_NAME` in `npm/react-refresh/index.js`).
/// The core crate has no `implemented_*_rule_names()` accessor and its
/// `Diagnostic` carries no `rule_name`, so the rule name is fixed here.
const RULE: &str = "only-export-components";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: vec![RULE.to_owned()],
    }
}

pub fn scan(
    source_text: &str,
    filename: &str,
    filter: &EnabledFilter,
    out: &mut Vec<PlaygroundDiagnostic>,
) {
    if !filter.rule_enabled(PLUGIN, RULE) {
        return;
    }

    // The playground passes no user options, matching `normalizeOptions(undefined)`
    // in the npm wrapper: empty `extraHOCs`/`allowExportNames`, both booleans false.
    // The core scan fn derives the source type from `filename` internally.
    let options = core::OnlyExportComponentsOptions::default();

    for diagnostic in core::scan_only_export_components(source_text, filename, &options) {
        // The core diagnostic exposes no data fields; `messages` placeholders are
        // all static text, so the data map is intentionally empty.
        let data: BTreeMap<&'static str, String> = BTreeMap::new();
        out.push(PlaygroundDiagnostic {
            plugin: PLUGIN,
            rule: RULE.to_owned(),
            message_id: diagnostic.message_id.to_owned(),
            data,
            start_line: diagnostic.loc.start_line,
            start_column: diagnostic.loc.start_column,
            end_line: diagnostic.loc.end_line,
            end_column: diagnostic.loc.end_column,
        });
    }
}
