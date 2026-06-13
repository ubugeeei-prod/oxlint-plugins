#![doc = "Rust implementation of eslint-plugin-functional rule logic."]
#![allow(
    clippy::disallowed_types,
    reason = "The first native functional port builds NAPI-facing diagnostics and small AST worklists; hot string data is compacted."
)]

mod helpers;
mod scanner;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::scanner::Scanner;

pub const RULE_NAMES: [&str; 20] = [
    "functional-parameters",
    "immutable-data",
    "no-class-inheritance",
    "no-classes",
    "no-conditional-statements",
    "no-expression-statements",
    "no-let",
    "no-loop-statements",
    "no-mixed-types",
    "no-promise-reject",
    "no-return-void",
    "no-this-expressions",
    "no-throw-statements",
    "no-try-statements",
    "prefer-immutable-types",
    "prefer-property-signatures",
    "prefer-readonly-type",
    "prefer-tacit",
    "readonly-type",
    "type-declaration-immutability",
];

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
pub struct FunctionalOptions {
    pub rule_names: SmallVec<[CompactString; 20]>,
    pub allow_rest_parameter: bool,
    pub allow_arguments_keyword: bool,
    pub allow_let_in_for_loop_init: bool,
    pub allow_throw_to_reject_promises: bool,
    pub allow_try_catch: bool,
    pub allow_try_finally: bool,
    pub readonly_type_mode: CompactString,
}

impl Default for FunctionalOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            allow_rest_parameter: false,
            allow_arguments_keyword: false,
            allow_let_in_for_loop_init: false,
            allow_throw_to_reject_promises: false,
            allow_try_catch: false,
            allow_try_finally: false,
            readonly_type_mode: "generic".into(),
        }
    }
}

impl FunctionalOptions {
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

#[derive(Clone, Copy)]
pub(crate) struct FunctionContext {
    pub(crate) in_async_function: bool,
}

pub fn implemented_functional_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_functional(
    source_text: &str,
    filename: &str,
    options: &FunctionalOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        options,
    };
    scanner.scan_statement_list(
        &parser_return.program.body,
        FunctionContext {
            in_async_function: false,
        },
    );
    scanner.diagnostics
}

#[cfg(test)]
mod tests;
