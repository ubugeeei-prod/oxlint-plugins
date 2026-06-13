#![doc = "Rust implementation of eslint-plugin-functional rule logic."]
#![allow(
    clippy::disallowed_types,
    reason = "The first native functional port builds NAPI-facing diagnostics and small AST worklists; hot string data is compacted."
)]

mod expressions;
mod helpers;
mod scanner;
mod statements;
mod types;

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
    /// The upstream eslint-plugin-functional messageId for this diagnostic, so
    /// the JS wrapper can report it and the upstream replay suite can assert it.
    pub message_id: &'static str,
    pub message: CompactString,
    pub loc: DiagnosticLoc,
}

/// Whether and how to enforce parameter counts for functional-parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum EnforceParameterCount {
    /// Do not enforce parameter count.
    Off,
    /// Every function must have at least one parameter.
    #[default]
    AtLeastOne,
    /// Every function must have exactly one parameter.
    ExactlyOne,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionalOptions {
    pub rule_names: SmallVec<[CompactString; 20]>,
    pub allow_rest_parameter: bool,
    pub allow_arguments_keyword: bool,
    pub allow_let_in_for_loop_init: bool,
    pub allow_in_functions: bool,
    pub allow_throw_to_reject_promises: bool,
    pub allow_try_catch: bool,
    pub allow_try_finally: bool,
    pub readonly_type_mode: CompactString,
    pub ignore_if_readonly_wrapped: bool,
    pub ignore_identifier_pattern: SmallVec<[CompactString; 4]>,
    pub ignore_code_pattern: SmallVec<[CompactString; 4]>,
    /// Whether/how to enforce parameter counts (default: AtLeastOne).
    pub enforce_parameter_count: EnforceParameterCount,
    /// When true, IIFEs are exempt from the parameter count check (default: true).
    pub enforce_count_ignore_iife: bool,
    /// When true, getters and setters are exempt from the parameter count check (default: true).
    pub enforce_count_ignore_getters_setters: bool,
    /// When true, lambda expressions (functions passed as arguments) are exempt (default: false).
    pub enforce_count_ignore_lambda: bool,
    /// Extracted method names from `ignorePrefixSelector` patterns of the form
    /// `CallExpression[callee.property.name='NAME']`.
    pub ignore_prefix_selector_names: SmallVec<[CompactString; 4]>,
    /// no-mixed-types: check `interface` declarations (default: true).
    pub check_interfaces: bool,
    /// no-mixed-types: check object type literals (default: true).
    pub check_type_literals: bool,
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
            allow_in_functions: false,
            allow_throw_to_reject_promises: false,
            allow_try_catch: false,
            allow_try_finally: false,
            readonly_type_mode: "generic".into(),
            ignore_if_readonly_wrapped: false,
            ignore_identifier_pattern: SmallVec::new(),
            ignore_code_pattern: SmallVec::new(),
            enforce_parameter_count: EnforceParameterCount::AtLeastOne,
            enforce_count_ignore_iife: true,
            enforce_count_ignore_getters_setters: true,
            enforce_count_ignore_lambda: false,
            ignore_prefix_selector_names: SmallVec::new(),
            check_interfaces: true,
            check_type_literals: true,
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

/// Metadata threaded into `scan_function`/`scan_arrow_function` so that
/// `scan_function_parameters` can apply the correct `functional-parameters`
/// ignore/count logic without a second traversal pass.
#[derive(Clone, Copy, Default)]
pub(crate) struct FunctionParamMeta<'a> {
    /// The declared name of the function, if any (for `ignoreIdentifierPattern`).
    pub name: Option<&'a str>,
    /// True when this function is the callee of a call expression (IIFE).
    pub is_iife: bool,
    /// True when this function is a getter or setter.
    pub is_getter_setter: bool,
    /// True when this function is passed directly as a call argument (lambda).
    pub is_lambda_arg: bool,
    /// The `callee.property.name` of the enclosing call when the function is a
    /// lambda arg, used for `ignorePrefixSelector` matching.
    pub enclosing_call_property: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub(crate) struct FunctionContext {
    pub(crate) in_async_function: bool,
    /// True when the current statement is inside the `try` block of a
    /// `try`/`catch` (a thrown value would be caught, so an async `throw` here is
    /// not a promise rejection). Reset at each function boundary.
    pub(crate) in_try_with_catch: bool,
    /// True when inside any function/arrow/method body; used by no-let allowInFunctions.
    pub(crate) in_function: bool,
    /// True when the enclosing function is a `.then`/`.catch` promise handler; an
    /// async-style throw there rejects the promise (no-throw allowToRejectPromises).
    pub(crate) in_promise_handler: bool,
}

pub fn implemented_functional_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

fn compile_patterns(patterns: &[CompactString]) -> SmallVec<[regex::Regex; 4]> {
    patterns
        .iter()
        .filter_map(|pattern| regex::Regex::new(pattern).ok())
        .collect()
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
        within_readonly: false,
        ignore_identifier_regexes: compile_patterns(&options.ignore_identifier_pattern),
        ignore_code_regexes: compile_patterns(&options.ignore_code_pattern),
    };
    scanner.scan_statement_list(
        &parser_return.program.body,
        FunctionContext {
            in_async_function: false,
            in_try_with_catch: false,
            in_function: false,
            in_promise_handler: false,
        },
    );
    scanner.diagnostics
}

#[cfg(test)]
mod tests;
