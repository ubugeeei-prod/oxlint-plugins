//! Public diagnostic types, options, and internal binding metadata for the
//! unused-imports port.

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
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message: CompactString,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnusedImportsOptions {
    pub rule_names: SmallVec<[CompactString; 2]>,
}

impl Default for UnusedImportsOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|name| CompactString::from(*name))
                .collect(),
        }
    }
}

impl UnusedImportsOptions {
    pub(crate) fn has_rule(&self, rule_name: &str) -> bool {
        self.rule_names.iter().any(|name| name == rule_name)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SpanKey {
    pub(crate) start: u32,
    pub(crate) end: u32,
}

impl From<Span> for SpanKey {
    fn from(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct DeclarationKey {
    pub(crate) start: u32,
    pub(crate) end: u32,
}

impl From<Span> for DeclarationKey {
    fn from(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ImportSpecifierKind {
    Named,
    Default,
    Namespace,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ImportBinding<'a> {
    pub(crate) name: &'a str,
    pub(crate) local_span: Span,
    pub(crate) specifier_span: Span,
    pub(crate) declaration_span: Span,
    pub(crate) specifier_index: usize,
    pub(crate) specifier_count: usize,
    pub(crate) named_specifier_count: usize,
    pub(crate) kind: ImportSpecifierKind,
}

#[derive(Default)]
pub(crate) struct DeclarationUsage {
    pub(crate) specifier_count: usize,
    pub(crate) unused_count: usize,
    pub(crate) first_unused_index: usize,
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

    pub(crate) fn line_for_offset(&self, offset: u32) -> u32 {
        let offset = offset as usize;
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        line_index.saturating_sub(1) as u32 + 1
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
