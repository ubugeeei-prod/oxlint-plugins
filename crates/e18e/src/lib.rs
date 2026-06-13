#![doc = "Rust implementation of @e18e/eslint-plugin rule logic."]
#![allow(
    clippy::collapsible_if,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::needless_borrow,
    clippy::question_mark,
    reason = "The e18e port builds many small autofix strings from source slices; keeping that string assembly local is clearer than adding broad formatting abstractions in the first native port."
)]

mod helpers;
mod rules;
mod scanner;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::SmallVec;

use crate::helpers::ban_dependency_diagnostic;
use crate::scanner::Scanner;
pub(crate) use crate::types::LineIndex;
pub use crate::types::{
    BanDependency, Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, E18eOptions,
};

pub const RULE_NAMES: [&str; 25] = [
    "prefer-array-at",
    "prefer-array-fill",
    "prefer-array-from-map",
    "prefer-includes",
    "prefer-array-to-reversed",
    "prefer-array-to-sorted",
    "prefer-array-to-spliced",
    "prefer-exponentiation-operator",
    "prefer-nullish-coalescing",
    "prefer-object-has-own",
    "prefer-spread-syntax",
    "prefer-url-canparse",
    "no-indexof-equality",
    "prefer-timer-args",
    "prefer-date-now",
    "prefer-regex-test",
    "prefer-array-some",
    "prefer-static-regex",
    "prefer-inline-equality",
    "prefer-string-fromcharcode",
    "prefer-includes-over-regex-test",
    "no-delete-property",
    "no-spread-in-reduce",
    "prefer-static-collator",
    "ban-dependencies",
];

pub fn implemented_e18e_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_e18e(
    source_text: &str,
    filename: &str,
    options: &E18eOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let line_index = LineIndex::new(source_text);
    if filename.ends_with("package.json") {
        return scan_package_json_dependencies(source_text, options, &line_index);
    }

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
        line_index,
        options,
        diagnostics: SmallVec::new(),
        function_depth: 0,
    };
    scanner.scan_program(&parser_return.program);
    scanner.diagnostics
}

fn scan_package_json_dependencies(
    source_text: &str,
    options: &E18eOptions,
    line_index: &LineIndex,
) -> SmallVec<[Diagnostic; 32]> {
    let mut diagnostics = SmallVec::new();
    if !options.has_rule("ban-dependencies") {
        return diagnostics;
    }

    for dependency in &options.banned_dependencies {
        let needle = format!("\"{}\"", dependency.module_name);
        let mut search_start = 0usize;
        while let Some(offset) = source_text[search_start..].find(&needle) {
            let start = search_start + offset;
            let span = Span::new(start as u32, (start + needle.len()) as u32);
            diagnostics.push(ban_dependency_diagnostic(
                dependency,
                span,
                source_text,
                line_index,
            ));
            search_start = start + needle.len();
        }
    }
    diagnostics
}
