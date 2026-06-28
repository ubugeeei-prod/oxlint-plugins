//! Per-plugin adapters that map each Rust core crate's diagnostics into the
//! unified [`PlaygroundDiagnostic`] shape, plus the registry that drives a run.

use std::collections::{BTreeMap, HashSet};

use oxlint_plugins_carton::CompactString;
use serde::Serialize;

use crate::{PlaygroundDiagnostic, PluginInfo};

/// Inserts a diagnostic data value into `data` under `key` when it is present.
/// Shared by every adapter so the data-mapping convention lives in one place.
pub(super) fn push(
    data: &mut BTreeMap<&'static str, String>,
    key: &'static str,
    value: Option<CompactString>,
) {
    if let Some(value) = value {
        data.insert(key, value.as_str().to_owned());
    }
}

mod angular_eslint;
mod cypress;
mod e18e;
mod eslint_comments;
mod eslint_json;
mod eslint_markdown;
mod functional;
mod mocha;
mod perfectionist;
mod playwright;
mod regexp;
mod security;
mod simple_import_sort;
mod sonarjs;
mod storybook;
mod stylistic;
mod testing_library;
mod unocss;
mod unused_imports;

/// Source language a plugin lints. Used to scope a plugin to matching files so,
/// e.g., the JSON plugin never runs against a `.js` file.
#[derive(Clone, Copy)]
enum Language {
    JavaScript,
    Json,
    Markdown,
}

impl Language {
    fn as_str(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::Json => "json",
            Self::Markdown => "markdown",
        }
    }

    /// Whether a file with the given extension should be linted by this language.
    fn matches_extension(self, ext: &str) -> bool {
        match self {
            Self::JavaScript => matches!(
                ext,
                "js" | "cjs" | "mjs" | "jsx" | "ts" | "cts" | "mts" | "tsx"
            ),
            Self::Json => matches!(ext, "json" | "jsonc" | "json5"),
            Self::Markdown => matches!(ext, "md" | "markdown"),
        }
    }
}

/// Every plugin adapter registered with the playground.
///
/// Each entry pairs the plugin's language and metadata accessor with the scan
/// entry point, so adding a plugin is a single line here plus its module.
type InfoFn = fn() -> PluginInfo;
type ScanFn = fn(&str, &str, &EnabledFilter, &mut Vec<PlaygroundDiagnostic>);

const REGISTRY: &[(&str, Language, InfoFn, ScanFn)] = &[
    (
        angular_eslint::PLUGIN,
        Language::JavaScript,
        angular_eslint::info,
        angular_eslint::scan,
    ),
    (
        cypress::PLUGIN,
        Language::JavaScript,
        cypress::info,
        cypress::scan,
    ),
    (e18e::PLUGIN, Language::JavaScript, e18e::info, e18e::scan),
    (
        eslint_json::PLUGIN,
        Language::Json,
        eslint_json::info,
        eslint_json::scan,
    ),
    (
        eslint_markdown::PLUGIN,
        Language::Markdown,
        eslint_markdown::info,
        eslint_markdown::scan,
    ),
    (
        functional::PLUGIN,
        Language::JavaScript,
        functional::info,
        functional::scan,
    ),
    (
        mocha::PLUGIN,
        Language::JavaScript,
        mocha::info,
        mocha::scan,
    ),
    (
        perfectionist::PLUGIN,
        Language::JavaScript,
        perfectionist::info,
        perfectionist::scan,
    ),
    (
        playwright::PLUGIN,
        Language::JavaScript,
        playwright::info,
        playwright::scan,
    ),
    (
        regexp::PLUGIN,
        Language::JavaScript,
        regexp::info,
        regexp::scan,
    ),
    (
        security::PLUGIN,
        Language::JavaScript,
        security::info,
        security::scan,
    ),
    (
        simple_import_sort::PLUGIN,
        Language::JavaScript,
        simple_import_sort::info,
        simple_import_sort::scan,
    ),
    (
        sonarjs::PLUGIN,
        Language::JavaScript,
        sonarjs::info,
        sonarjs::scan,
    ),
    (
        storybook::PLUGIN,
        Language::JavaScript,
        storybook::info,
        storybook::scan,
    ),
    (
        stylistic::PLUGIN,
        Language::JavaScript,
        stylistic::info,
        stylistic::scan,
    ),
    (
        testing_library::PLUGIN,
        Language::JavaScript,
        testing_library::info,
        testing_library::scan,
    ),
    (
        unocss::PLUGIN,
        Language::JavaScript,
        unocss::info,
        unocss::scan,
    ),
    (
        unused_imports::PLUGIN,
        Language::JavaScript,
        unused_imports::info,
        unused_imports::scan,
    ),
    // eslint-comments runs last so its `no-unused-disable` rule can treat the
    // other plugins' diagnostics as the file's lint problems.
    (
        eslint_comments::PLUGIN,
        Language::JavaScript,
        eslint_comments::info,
        eslint_comments::scan,
    ),
];

/// One plugin entry in the `list_rules` payload sent to the UI.
#[derive(Serialize)]
pub struct PluginListing {
    pub plugin: &'static str,
    pub language: &'static str,
    pub rules: Vec<String>,
}

/// Returns the stylistic rule metas (names, descriptions, message templates) as
/// JSON, used by the catalog build since stylistic messages live in Rust.
pub fn stylistic_rule_metas() -> String {
    stylistic::rule_metas_json()
}

/// Returns metadata for every plugin the playground can run.
pub fn list_plugins() -> Vec<PluginListing> {
    REGISTRY
        .iter()
        .map(|(_, language, info, _)| {
            let info = info();
            PluginListing {
                plugin: info.plugin,
                language: language.as_str(),
                rules: info.rules,
            }
        })
        .collect()
}

/// Returns the lowercased-friendly extension of `filename` (without the dot).
fn extension(filename: &str) -> &str {
    match filename.rsplit_once('.') {
        Some((_, ext)) => ext,
        None => "",
    }
}

/// Returns the source language for a file name (`javascript`, `json`,
/// `markdown`, or `""`). The UI calls this so the editor and the rule scoping
/// share one authoritative extension map.
pub fn language_for_filename(filename: &str) -> &'static str {
    let ext = extension(filename).to_ascii_lowercase();
    for language in [Language::JavaScript, Language::Json, Language::Markdown] {
        if language.matches_extension(&ext) {
            return language.as_str();
        }
    }
    ""
}

/// Runs the enabled plugins over `source_text` and collects diagnostics.
///
/// A plugin only runs when it is enabled AND its language matches the file
/// extension, mirroring how each plugin's recommended config scopes itself.
pub fn run(source_text: &str, filename: &str, filter: &EnabledFilter) -> Vec<PlaygroundDiagnostic> {
    let ext = extension(filename).to_ascii_lowercase();
    let mut diagnostics = Vec::new();
    for (plugin, language, _, scan) in REGISTRY {
        if filter.plugin_enabled(plugin) && language.matches_extension(&ext) {
            scan(source_text, filename, filter, &mut diagnostics);
        }
    }
    diagnostics
}

/// Describes which plugins and rules are active for a lint run.
pub struct EnabledFilter {
    /// `None` means "every plugin and rule is enabled".
    map: Option<BTreeMap<String, RuleSet>>,
}

enum RuleSet {
    All,
    Some(HashSet<String>),
}

impl EnabledFilter {
    /// Parses the JSON object passed from the UI. Any parse failure or empty
    /// object enables everything so the playground stays useful by default.
    pub fn parse(enabled_json: &str) -> Self {
        let trimmed = enabled_json.trim();
        if trimmed.is_empty() {
            return Self { map: None };
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            return Self { map: None };
        };
        let serde_json::Value::Object(object) = value else {
            return Self { map: None };
        };
        if object.is_empty() {
            return Self { map: None };
        }
        let mut map = BTreeMap::new();
        for (plugin, rules) in object {
            let set = match rules {
                serde_json::Value::Array(items) => RuleSet::Some(
                    items
                        .into_iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect(),
                ),
                _ => RuleSet::All,
            };
            map.insert(plugin, set);
        }
        Self { map: Some(map) }
    }

    fn plugin_enabled(&self, plugin: &str) -> bool {
        match &self.map {
            None => true,
            Some(map) => map.contains_key(plugin),
        }
    }

    /// Returns whether `rule` of `plugin` should be reported.
    pub fn rule_enabled(&self, plugin: &str, rule: &str) -> bool {
        match &self.map {
            None => true,
            Some(map) => match map.get(plugin) {
                None => false,
                Some(RuleSet::All) => true,
                Some(RuleSet::Some(rules)) => rules.contains(rule),
            },
        }
    }
}
