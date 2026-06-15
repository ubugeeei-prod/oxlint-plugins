//! Public-facing diagnostic and option types for the e18e port.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::RULE_NAMES;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub array: Option<CompactString>,
    pub index: Option<CompactString>,
    pub item: Option<CompactString>,
    pub length: Option<CompactString>,
    pub value: Option<CompactString>,
    pub iterable: Option<CompactString>,
    pub mapper: Option<CompactString>,
    pub regex: Option<CompactString>,
    pub string: Option<CompactString>,
    pub original: Option<CompactString>,
    pub name: Option<CompactString>,
    pub replacement: Option<CompactString>,
    pub url: Option<CompactString>,
    pub description: Option<CompactString>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BanDependency {
    pub module_name: CompactString,
    pub message_id: CompactString,
    pub replacement: Option<CompactString>,
    pub url: Option<CompactString>,
    pub description: Option<CompactString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct E18eOptions {
    pub rule_names: SmallVec<[CompactString; 25]>,
    pub banned_dependencies: SmallVec<[BanDependency; 16]>,
}

impl Default for E18eOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            banned_dependencies: SmallVec::new(),
        }
    }
}

impl E18eOptions {
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
