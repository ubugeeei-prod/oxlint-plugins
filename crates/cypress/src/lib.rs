#![doc = "Rust implementation of eslint-plugin-cypress rule logic."]

mod helpers;
mod scanner;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};

use crate::scanner::Scanner;

pub const RULE_NAMES: [&str; 13] = [
    "assertion-before-screenshot",
    "no-and",
    "no-assigning-return-values",
    "no-async-before",
    "no-async-tests",
    "no-chained-get",
    "no-debug",
    "no-force",
    "no-pause",
    "no-unnecessary-waiting",
    "no-xpath",
    "require-data-selectors",
    "unsafe-to-chain-command",
];

pub(crate) const ASSERTION_COMMANDS: [&str; 6] = [
    "should",
    "and",
    "contains",
    "get",
    "scrollIntoView",
    "scrollTo",
];
pub(crate) const ALLOW_AND_AFTER: [&str; 3] = ["should", "and", "contains"];
pub(crate) const ASSIGNMENT_ALLOWED_COMMANDS: [&str; 4] = ["now", "spy", "state", "stub"];
pub(crate) const FORCE_ACTION_COMMANDS: [&str; 8] = [
    "click",
    "dblclick",
    "type",
    "trigger",
    "check",
    "rightclick",
    "focus",
    "select",
];
pub(crate) const UNSAFE_CHAIN_ACTIONS: [&str; 19] = [
    "blur",
    "clear",
    "click",
    "check",
    "dblclick",
    "each",
    "focus",
    "rightclick",
    "screenshot",
    "scrollIntoView",
    "scrollTo",
    "select",
    "selectFile",
    "spread",
    "submit",
    "type",
    "trigger",
    "uncheck",
    "within",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CypressOptions {
    pub unsafe_to_chain_methods: SmallVec<[CompactString; 8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValueKind {
    Number,
    Other,
}

#[derive(Default)]
pub(crate) struct Scope {
    pub(crate) values: FastHashMap<CompactString, ValueKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParentKind {
    None,
    MemberObject,
    Other,
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

pub fn implemented_cypress_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_cypress(
    source_text: &str,
    filename: &str,
    options: &CypressOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        scopes: SmallVec::new(),
        data_selector_variables: FastHashMap::default(),
        unsafe_to_chain_methods: options.unsafe_to_chain_methods.clone(),
    };
    scanner.push_scope();
    scanner.scan_statement_list(&parser_return.program.body, None, false);
    scanner.diagnostics
}

#[cfg(test)]
mod tests;
