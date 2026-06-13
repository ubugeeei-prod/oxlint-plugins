//! Diagnostic types, options, and small scanner state structs for the unocss
//! port.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

pub(crate) const DEFAULT_UNO_FUNCTIONS: [&str; 2] = ["clsx", "classnames"];
pub(crate) const DEFAULT_UNO_VARIABLES: [&str; 2] = ["^cls", "classNames?$"];

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
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
    pub name: Option<CompactString>,
    pub reason: Option<CompactString>,
    pub prefix: Option<CompactString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlocklistEntry {
    pub name: CompactString,
    pub reason: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnocssOptions {
    pub uno_functions: SmallVec<[CompactString; 4]>,
    pub uno_variables: SmallVec<[CompactString; 4]>,
    pub blocklist: SmallVec<[BlocklistEntry; 4]>,
    pub class_compile_prefix: CompactString,
    pub class_compile_enable_fix: bool,
}

impl Default for UnocssOptions {
    fn default() -> Self {
        Self {
            uno_functions: DEFAULT_UNO_FUNCTIONS
                .into_iter()
                .map(CompactString::from)
                .collect(),
            uno_variables: DEFAULT_UNO_VARIABLES
                .into_iter()
                .map(CompactString::from)
                .collect(),
            blocklist: SmallVec::new(),
            class_compile_prefix: CompactString::from(":uno:"),
            class_compile_enable_fix: true,
        }
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

#[derive(Clone, Copy)]
pub(crate) struct LiteralSpan<'a> {
    pub(crate) full_start: usize,
    pub(crate) content_start: usize,
    pub(crate) content_end: usize,
    pub(crate) content: &'a str,
}

#[derive(Clone)]
pub(crate) struct TokenPart<'a> {
    pub(crate) text: &'a str,
    pub(crate) index: usize,
}

#[derive(Default)]
pub(crate) struct ReportData {
    pub(crate) fix: Option<DiagnosticFix>,
    pub(crate) name: Option<CompactString>,
    pub(crate) reason: Option<CompactString>,
    pub(crate) prefix: Option<CompactString>,
}
