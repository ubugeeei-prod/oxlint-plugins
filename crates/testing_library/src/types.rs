//! Diagnostic types, options, and line indexing for the testing-library port.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::RULE_NAMES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message: CompactString,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestingLibraryOptions {
    pub rule_names: SmallVec<[CompactString; 29]>,
    pub test_id_pattern: CompactString,
}

impl Default for TestingLibraryOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            test_id_pattern: CompactString::from("kebab-case"),
        }
    }
}

impl TestingLibraryOptions {
    pub(crate) fn has_rule(&self, rule_name: &str) -> bool {
        self.rule_names.iter().any(|name| name == rule_name)
    }
}

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
