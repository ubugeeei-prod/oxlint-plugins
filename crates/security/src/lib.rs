#![doc = "Rust implementation of eslint-plugin-security rule logic."]

mod helpers;
mod scanner;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};

use crate::scanner::Scanner;

pub const RULE_NAMES: [&str; 14] = [
    "detect-bidi-characters",
    "detect-buffer-noassert",
    "detect-child-process",
    "detect-disable-mustache-escape",
    "detect-eval-with-expression",
    "detect-new-buffer",
    "detect-no-csrf-before-method-override",
    "detect-non-literal-fs-filename",
    "detect-non-literal-regexp",
    "detect-non-literal-require",
    "detect-object-injection",
    "detect-possible-timing-attacks",
    "detect-pseudoRandomBytes",
    "detect-unsafe-regex",
];

pub(crate) const CHILD_PROCESS_PACKAGES: [&str; 2] = ["child_process", "node:child_process"];
pub(crate) const FS_PACKAGES: [&str; 5] = [
    "fs",
    "node:fs",
    "fs/promises",
    "node:fs/promises",
    "fs-extra",
];
pub(crate) const PATH_PACKAGES: [&str; 4] = ["path", "node:path", "path/posix", "node:path/posix"];
pub(crate) const URL_PACKAGES: [&str; 2] = ["url", "node:url"];
pub(crate) const PATH_CONSTRUCTION_METHODS: [&str; 8] = [
    "basename",
    "dirname",
    "extname",
    "join",
    "normalize",
    "relative",
    "resolve",
    "toNamespacedPath",
];
pub(crate) const PATH_STATIC_MEMBERS: [&str; 2] = ["delimiter", "sep"];
pub(crate) const TIMING_KEYWORDS: [&str; 8] = [
    "password", "secret", "api", "apikey", "token", "auth", "pass", "hash",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub text: Option<CompactString>,
    pub method: Option<CompactString>,
    pub package_name: Option<CompactString>,
    pub fn_name: Option<CompactString>,
    pub indices: Option<CompactString>,
    pub side: Option<CompactString>,
    pub value: Option<CompactString>,
    pub argument_type: Option<CompactString>,
}

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
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AccessPath {
    pub(crate) package_name: CompactString,
    pub(crate) path: SmallVec<[CompactString; 4]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Binding {
    Unknown,
    Static,
    Import(AccessPath),
}

#[derive(Default)]
pub(crate) struct Scope {
    pub(crate) bindings: FastHashMap<CompactString, Binding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParentKind {
    None,
    VariableInit,
    AssignmentRight,
    AssignmentLeft,
    MemberObject,
    CallCallee,
    CallArgument,
    NewCallee,
    NewArgument,
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

pub fn implemented_security_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_security(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let line_index = LineIndex::new(source_text);
    let mut scanner = Scanner {
        source_text,
        line_index,
        diagnostics: SmallVec::new(),
        scopes: SmallVec::new(),
        csrf_seen: false,
        comment_spans: parser_return
            .program
            .comments
            .iter()
            .map(|comment| comment.span)
            .collect(),
    };
    scanner.push_scope();
    scanner.scan_bidi_characters();
    scanner.scan_program(&parser_return.program.body);
    scanner.diagnostics
}

#[cfg(test)]
mod tests;
