#![doc = "Rust implementation of eslint-plugin-react-refresh rule logic."]

mod helpers;
mod scanner;

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::scanner::Scanner;

/// Built-in higher-order components accepted by `only-export-components`.
pub const DEFAULT_HOCS: [&str; 3] = ["memo", "forwardRef", "lazy"];

/// Expression node kinds that can be exported alongside components when
/// `allowConstantExport` is enabled.
pub const CONSTANT_EXPORT_EXPRESSION_KINDS: [&str; 4] = [
    "Literal",
    "UnaryExpression",
    "TemplateLiteral",
    "BinaryExpression",
];

#[derive(Debug, Default)]
pub struct OnlyExportComponentsOptions {
    pub extra_hocs: SmallVec<[CompactString; 4]>,
    pub allow_export_names: SmallVec<[CompactString; 8]>,
    pub allow_constant_export: bool,
    pub check_js: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Copy)]
pub(crate) struct NamedSpan<'a> {
    pub(crate) name: &'a str,
    pub(crate) span: Span,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ComponentCheck {
    Yes,
    No,
    NeedName,
}

#[derive(Default)]
pub(crate) struct ScanState {
    pub(crate) has_exports: bool,
    pub(crate) has_react_export: bool,
    pub(crate) react_is_in_scope: bool,
    pub(crate) local_components: SmallVec<[Span; 8]>,
    pub(crate) non_component_exports: SmallVec<[Span; 8]>,
    pub(crate) react_context_exports: SmallVec<[Span; 4]>,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 8]>,
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

/// Returns true when `name` follows React component PascalCase naming.
pub fn is_react_component_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Returns true for filenames intentionally skipped by upstream.
pub fn is_excluded_test_like_filename(filename: &str) -> bool {
    [".test.", ".spec.", ".cy.", ".stories."]
        .iter()
        .any(|needle| filename.contains(needle))
}

/// Returns true when the rule should inspect the file.
pub fn should_scan_filename(filename: &str, check_js: bool) -> bool {
    if is_excluded_test_like_filename(filename) {
        return false;
    }

    filename.ends_with(".jsx")
        || filename.ends_with(".tsx")
        || (check_js && filename.ends_with(".js"))
}

pub fn is_constant_export_expression_kind(kind: &str) -> bool {
    CONSTANT_EXPORT_EXPRESSION_KINDS.contains(&kind)
}

pub fn default_hocs() -> SmallVec<[&'static str; 3]> {
    DEFAULT_HOCS.into_iter().collect()
}

pub fn scan_only_export_components(
    source_text: &str,
    filename: &str,
    options: &OnlyExportComponentsOptions,
) -> SmallVec<[Diagnostic; 8]> {
    if !should_scan_filename(filename, options.check_js) {
        return SmallVec::new();
    }

    let allocator = Allocator::default();
    let source_type = source_type_for_filename(filename, options.check_js);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let line_index = LineIndex::new(source_text);
    let mut scanner = Scanner {
        line_index: &line_index,
        options,
        source_text,
        state: ScanState::default(),
    };
    scanner.scan_program(&parser_return.program.body);
    scanner.finish()
}

fn source_type_for_filename(filename: &str, check_js: bool) -> SourceType {
    if filename.ends_with(".tsx") {
        SourceType::tsx()
    } else if filename.ends_with(".jsx") || (check_js && filename.ends_with(".js")) {
        SourceType::jsx()
    } else {
        SourceType::from_path(Path::new(filename)).unwrap_or_else(|_| SourceType::mjs())
    }
}

#[cfg(test)]
mod tests;
