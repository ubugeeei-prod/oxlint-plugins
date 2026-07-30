#![doc = "Rust implementation of eslint-plugin-perfectionist rule logic."]

mod helpers;
mod scanner;
mod sort_array_includes;
mod sort_named_specifiers;
mod sort_options;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;
use crate::types::LineIndex;

pub use crate::types::{
    Diagnostic, DiagnosticLoc, RuleDiagnostic, RuleDiagnosticData, RuleDiagnosticFix,
};

pub const RULE_NAMES: [&str; 23] = [
    "sort-array-includes",
    "sort-arrays",
    "sort-classes",
    "sort-decorators",
    "sort-enums",
    "sort-export-attributes",
    "sort-exports",
    "sort-heritage-clauses",
    "sort-import-attributes",
    "sort-imports",
    "sort-interfaces",
    "sort-intersection-types",
    "sort-jsx-props",
    "sort-maps",
    "sort-modules",
    "sort-named-exports",
    "sort-named-imports",
    "sort-object-types",
    "sort-objects",
    "sort-sets",
    "sort-switch-case",
    "sort-union-types",
    "sort-variable-declarations",
];

pub fn implemented_perfectionist_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_perfectionist(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 24]> {
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
    };
    scanner.scan();
    scanner.diagnostics
}

pub fn scan_perfectionist_rule(
    source_text: &str,
    filename: &str,
    rule_name: &str,
    options: &serde_json::Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }
    match rule_name {
        "sort-array-includes" => sort_array_includes::check(
            source_text,
            &parser_return.program,
            &parser_return.program.comments,
            options,
        ),
        "sort-named-imports" => sort_named_specifiers::check(
            source_text,
            &parser_return.program.body,
            &parser_return.program.comments,
            options,
        ),
        "sort-named-exports" => sort_named_specifiers::check_exports(
            source_text,
            &parser_return.program.body,
            &parser_return.program.comments,
            options,
        ),
        "sort-exports" => sort_named_specifiers::check_sort_exports(
            source_text,
            &parser_return.program.body,
            &parser_return.program.comments,
            options,
        ),
        "sort-imports" => sort_named_specifiers::check_sort_imports(
            source_text,
            &parser_return.program.body,
            &parser_return.program.comments,
            options,
        ),
        _ => SmallVec::new(),
    }
}
