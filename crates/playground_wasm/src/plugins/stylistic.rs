//! Adapter for the `stylistic` plugin (port of @stylistic/eslint-plugin).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use oxlint_plugins_stylistic as core;
use serde_json::Value;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "stylistic";

/// Stylistic has no `implemented_*_rule_names()` helper, so we derive the names
/// from the rule metas once and cache them — `scan` runs on every keystroke and
/// shouldn't rebuild the full metadata vector each time.
fn rule_names() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        core::stylistic_rule_metas()
            .into_iter()
            .map(|meta| meta.name)
            .collect()
    })
}

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: rule_names().to_vec(),
    }
}

/// Stylistic renders its messages in Rust, so the templates live in the rule
/// metas rather than in `index.js`. The catalog build reads this to show full
/// messages and descriptions for stylistic rules.
pub fn rule_metas_json() -> String {
    serde_json::to_string(&core::stylistic_rule_metas()).unwrap_or_else(|_| "[]".to_string())
}

pub fn scan(
    source_text: &str,
    _filename: &str,
    filter: &EnabledFilter,
    out: &mut Vec<PlaygroundDiagnostic>,
) {
    // The NAPI wrapper batches per-rule options into one native pass via
    // `runNativeStylisticLint(sourceText, { rules: config })`. The playground has
    // no per-rule configuration, so we mirror the wrapper's default path (empty
    // options, the `currentRuleOptions` default when `context.options` is empty)
    // but pass only the enabled rules to the core so it never runs disabled ones.
    // Stylistic works on raw bytes and lines, so `filename` is unused.
    let config = core::StylisticRunConfig {
        rules: rule_names()
            .iter()
            .filter(|name| filter.rule_enabled(PLUGIN, name))
            .map(|name| core::StylisticRuleConfig {
                name: name.clone(),
                options: Value::Array(Vec::new()),
            })
            .collect(),
    };
    if config.rules.is_empty() {
        return;
    }

    // `run_stylistic_lint` returns `Result<_, String>`; the npm wrapper surfaces
    // the error to JS. The playground simply reports nothing on failure.
    let Ok(diagnostics) = core::run_stylistic_lint(source_text, &config) else {
        return;
    };

    let line_starts = line_starts(source_text);
    for diagnostic in diagnostics {
        // Stylistic renders each message in Rust and exposes no placeholder data,
        // and every message template is a fixed string (no `{{placeholder}}`), so
        // the data map is always empty.
        let data: BTreeMap<&'static str, String> = BTreeMap::new();
        // The core diagnostic carries a UTF-8 byte range rather than line/column
        // loc fields. Convert both ends to the playground's 1-based line /
        // 0-based UTF-16 column convention.
        let (start_line, start_column) =
            position_for_offset(source_text, &line_starts, diagnostic.range.start);
        let (end_line, end_column) =
            position_for_offset(source_text, &line_starts, diagnostic.range.end);
        out.push(PlaygroundDiagnostic {
            plugin: PLUGIN,
            rule: diagnostic.rule_name,
            message_id: diagnostic.message_id,
            data,
            start_line,
            start_column,
            end_line,
            end_column,
        });
    }
}

/// Byte offsets of each line start (the first line starts at offset 0).
fn line_starts(source_text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    starts.push(0);
    for (index, ch) in source_text.char_indices() {
        if ch == '\n' {
            starts.push(index + 1);
        }
    }
    starts
}

/// Maps a UTF-8 byte `offset` to a `(line, column)` pair, where lines are
/// 1-based and columns are 0-based UTF-16 code units from the line start.
fn position_for_offset(source_text: &str, line_starts: &[usize], offset: u32) -> (u32, u32) {
    let offset = (offset as usize).min(source_text.len());
    let line_index = line_starts.partition_point(|start| *start <= offset);
    let line_index = line_index.saturating_sub(1);
    let line_start = line_starts.get(line_index).copied().unwrap_or(0);
    let column = source_text
        .get(line_start..offset)
        .unwrap_or("")
        .chars()
        .map(char::len_utf16)
        .sum::<usize>();
    ((line_index + 1) as u32, column as u32)
}
