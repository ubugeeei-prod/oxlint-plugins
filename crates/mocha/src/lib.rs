#![doc = "Rust implementation of eslint-plugin-mocha rule logic."]

mod checks;
mod expressions;
mod helpers;
mod scanner;
mod statements;

use oxc_allocator::Allocator;
use oxc_ast::ast::FunctionBody;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};
use regex::Regex;

use crate::scanner::Scanner;

pub const RULE_NAMES: [&str; 24] = [
    "consistent-interface",
    "consistent-spacing-between-blocks",
    "handle-done-callback",
    "max-top-level-suites",
    "no-async-suite",
    "no-empty-title",
    "no-exclusive-tests",
    "no-exports",
    "no-global-tests",
    "no-hooks",
    "no-hooks-for-single-case",
    "no-identical-title",
    "no-mocha-arrows",
    "no-nested-tests",
    "no-pending-tests",
    "no-return-and-callback",
    "no-return-from-async",
    "no-setup-in-describe",
    "no-sibling-hooks",
    "no-synchronous-tests",
    "no-top-level-hooks",
    "prefer-arrow-callback",
    "valid-suite-title",
    "valid-test-title",
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
pub struct MochaOptions {
    pub consistent_interface: CompactString,
    pub max_top_level_suites_limit: u32,
    pub handle_done_ignore_pending: bool,
    pub no_hooks_allowed: SmallVec<[CompactString; 4]>,
    pub no_hooks_for_single_case_allowed: SmallVec<[CompactString; 4]>,
    pub no_synchronous_allowed: SmallVec<[CompactString; 3]>,
    pub no_empty_title_message: Option<CompactString>,
    pub valid_suite_title_pattern: Option<CompactString>,
    pub valid_suite_title_message: Option<CompactString>,
    pub valid_test_title_pattern: Option<CompactString>,
    pub valid_test_title_message: Option<CompactString>,
    pub prefer_arrow_allow_named_functions: bool,
    pub prefer_arrow_allow_unbound_this: bool,
}

impl Default for MochaOptions {
    fn default() -> Self {
        let mut no_synchronous_allowed = SmallVec::new();
        no_synchronous_allowed.push("async".into());
        no_synchronous_allowed.push("callback".into());
        no_synchronous_allowed.push("promise".into());
        Self {
            consistent_interface: "BDD".into(),
            max_top_level_suites_limit: 1,
            handle_done_ignore_pending: false,
            no_hooks_allowed: SmallVec::new(),
            no_hooks_for_single_case_allowed: SmallVec::new(),
            no_synchronous_allowed,
            no_empty_title_message: None,
            valid_suite_title_pattern: None,
            valid_suite_title_message: None,
            valid_test_title_pattern: None,
            valid_test_title_message: None,
            prefer_arrow_allow_named_functions: false,
            prefer_arrow_allow_unbound_this: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntityType {
    Suite,
    TestCase,
    Hook,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MochaInterface {
    Bdd,
    Tdd,
}

impl MochaInterface {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Bdd => "BDD",
            Self::Tdd => "TDD",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Modifier {
    Pending,
    Exclusive,
}

#[derive(Clone, Copy)]
pub(crate) struct Callback<'a> {
    pub(crate) span: Span,
    pub(crate) body: CallbackBody<'a>,
    pub(crate) async_function: bool,
    pub(crate) arrow: bool,
    pub(crate) named_function: bool,
    pub(crate) params_len: usize,
    pub(crate) first_param_name: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub(crate) enum CallbackBody<'a> {
    Function(&'a FunctionBody<'a>),
}

#[derive(Clone)]
pub(crate) struct Entity<'a> {
    pub(crate) name: CompactString,
    pub(crate) entity_type: EntityType,
    pub(crate) interface: MochaInterface,
    pub(crate) modifier: Option<Modifier>,
    pub(crate) span: Span,
    pub(crate) title: Option<CompactString>,
    pub(crate) callback: Option<Callback<'a>>,
}

#[derive(Default)]
pub(crate) struct Layer {
    pub(crate) suite_titles: FastHashMap<CompactString, Span>,
    pub(crate) test_titles: FastHashMap<CompactString, Span>,
    pub(crate) hook_names: FastHashMap<CompactString, Span>,
    pub(crate) hooks: SmallVec<[(CompactString, Span); 4]>,
    pub(crate) test_count: u32,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContextKind {
    Program,
    SuiteCallback,
    TestCallback,
    HookCallback,
    Other,
}

pub fn implemented_mocha_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_mocha(
    source_text: &str,
    filename: &str,
    options: &MochaOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let valid_suite_regex = options
        .valid_suite_title_pattern
        .as_ref()
        .and_then(|pattern| Regex::new(pattern.as_str()).ok());
    let valid_test_regex = options
        .valid_test_title_pattern
        .as_ref()
        .and_then(|pattern| Regex::new(pattern.as_str()).ok());
    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        options,
        valid_suite_regex,
        valid_test_regex,
        layers: SmallVec::new(),
        suite_depth: 0,
        test_depth: 0,
        top_level_suites: 0,
        has_test_entity: false,
        export_spans: SmallVec::new(),
    };
    scanner.layers.push(Layer::default());
    scanner.scan_statement_list(&parser_return.program.body, ContextKind::Program, true);
    scanner.finish_program();
    scanner.diagnostics
}

#[cfg(test)]
mod tests;
