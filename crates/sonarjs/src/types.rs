//! Public-facing diagnostic and option types for the sonarjs port.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::RULE_NAMES;

/// Interpolation values forwarded to the JavaScript message templates.
///
/// Message text lives in the JS adapter (`npm/sonarjs/index.js`); the Rust core
/// only emits a `message_id` plus any placeholder values. Fields are added as
/// rules need them.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub value: Option<CompactString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

/// Per-scan options: the set of enabled rule names plus per-rule thresholds
/// that mirror the configurable options of the corresponding SonarJS rules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SonarjsOptions {
    pub rule_names: SmallVec<[CompactString; 32]>,
    /// Threshold for `max-lines` (S104); the SonarJS default is 1000.
    pub max_lines_threshold: u32,
    /// Threshold for `max-lines-per-function` (S138); the SonarJS default is 200.
    pub max_lines_per_function_threshold: u32,
    /// Threshold for `max-switch-cases` (S1479); the SonarJS default is 30.
    pub max_switch_cases_threshold: u32,
    /// Threshold for `max-union-size` (S4622); the SonarJS default is 3.
    pub max_union_size_threshold: u32,
    /// Maximum nesting level for `nested-control-flow` (S134); the SonarJS default is 3.
    pub nested_control_flow_threshold: u32,
    /// Threshold for `no-duplicate-string` (S1192); the SonarJS default is 3.
    pub no_duplicate_string_threshold: u32,
}

impl Default for SonarjsOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            max_lines_threshold: 1000,
            max_lines_per_function_threshold: 200,
            max_switch_cases_threshold: 30,
            max_union_size_threshold: 3,
            nested_control_flow_threshold: 3,
            no_duplicate_string_threshold: 3,
        }
    }
}

impl SonarjsOptions {
    pub(crate) fn has_rule(&self, rule_name: &str) -> bool {
        self.rule_names.iter().any(|name| name == rule_name)
    }
}

/// Maps byte offsets to 1-based lines and 0-based UTF-16 columns, matching the
/// `loc` convention expected by Oxlint/ESLint reports.
pub(crate) struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    pub(crate) fn new(source_text: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    pub(crate) fn loc_for_span(&self, source_text: &str, span: Span) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source_text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        let line_index = line_index.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = source_text[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line_index + 1) as u32, column as u32)
    }
}
