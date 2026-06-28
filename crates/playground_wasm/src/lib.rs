#![doc = "WebAssembly aggregator that runs the Rust-backed oxlint plugin rule logic in the browser."]
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "This crate is the wasm-bindgen/serde ABI boundary that exposes the core rule logic to the browser; std String/Vec/BTreeMap, to_string, and vec! are required by the public ABI and serialization, mirroring the NAPI wrapper crates."
)]

use std::collections::BTreeMap;

use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

mod plugins;

/// A single lint diagnostic in the unified shape consumed by the playground UI.
#[derive(Serialize)]
pub struct PlaygroundDiagnostic {
    pub plugin: &'static str,
    pub rule: String,
    pub message_id: String,
    pub data: BTreeMap<&'static str, String>,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Metadata describing one plugin and the rules it implements.
pub struct PluginInfo {
    pub plugin: &'static str,
    pub rules: Vec<String>,
}

/// Returns every implemented plugin and its rule names as a JSON string.
#[wasm_bindgen]
pub fn list_rules() -> String {
    let plugins = plugins::list_plugins();
    serde_json::to_string(&plugins).unwrap_or_else(|_| "[]".to_owned())
}

/// Returns the stylistic rule metadata as JSON. Stylistic renders messages in
/// Rust, so the playground catalog reads templates and descriptions from here.
#[wasm_bindgen]
pub fn stylistic_rule_metas() -> String {
    plugins::stylistic_rule_metas()
}

/// Returns the source language for `filename` (`javascript`, `json`, `markdown`,
/// or `""`), so the UI's editor and the rule scoping use one extension map.
#[wasm_bindgen]
pub fn language_for_filename(filename: &str) -> String {
    plugins::language_for_filename(filename).to_owned()
}

/// Lints `source_text` and returns the diagnostics as a JSON string.
///
/// `filename` controls the inferred source type (e.g. `.tsx`). `enabled_json`
/// is a JSON object mapping a plugin name to either `true` (all rules) or an
/// array of rule names; an empty or invalid value enables every plugin.
#[wasm_bindgen]
pub fn lint(source_text: &str, filename: &str, enabled_json: &str) -> String {
    let filter = plugins::EnabledFilter::parse(enabled_json);
    let diagnostics = plugins::run(source_text, filename, &filter);
    serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_owned())
}
