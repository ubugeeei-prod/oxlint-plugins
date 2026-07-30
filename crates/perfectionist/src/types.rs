//! Diagnostic types and line indexing for the perfectionist port.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

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
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleDiagnosticData {
    pub left: CompactString,
    pub right: CompactString,
    pub left_group: Option<CompactString>,
    pub right_group: Option<CompactString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleDiagnosticFix {
    /// UTF-16 source offset expected by ESLint/Oxlint fixers.
    pub start: u32,
    /// UTF-16 source offset expected by ESLint/Oxlint fixers.
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleDiagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub data: RuleDiagnosticData,
    pub loc: DiagnosticLoc,
    pub fix: RuleDiagnosticFix,
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

    pub(crate) fn utf16_offset(source_text: &str, byte_offset: u32) -> u32 {
        let byte_offset = (byte_offset as usize).min(source_text.len());
        let units = source_text[..byte_offset].encode_utf16().count();
        u32::try_from(units).unwrap_or(u32::MAX)
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
