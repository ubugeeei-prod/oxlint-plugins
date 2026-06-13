#![doc = "Rust implementation of selected eslint-plugin-regexp rule logic."]

mod checks;
mod expressions;
mod helpers;
mod pattern;
mod scanner;
mod traversal;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;
use crate::types::LineIndex;

pub use crate::types::{Diagnostic, DiagnosticData, DiagnosticLoc};

pub const RULE_NAMES: [&str; 35] = [
    "no-invalid-regexp",
    "no-empty-character-class",
    "no-empty-group",
    "no-empty-capturing-group",
    "no-empty-alternative",
    "no-zero-quantifier",
    "no-octal",
    "no-control-character",
    "sort-flags",
    "require-unicode-regexp",
    "no-escape-backspace",
    "prefer-plus-quantifier",
    "prefer-star-quantifier",
    "prefer-question-quantifier",
    "no-useless-two-nums-quantifier",
    "prefer-named-capture-group",
    "match-any",
    "no-legacy-features",
    "prefer-d",
    "prefer-w",
    "letter-case",
    "no-non-standard-flag",
    "no-invisible-character",
    "hexadecimal-escape",
    "unicode-escape",
    "no-useless-range",
    "no-empty-lookarounds-assertion",
    "prefer-regexp-exec",
    "no-missing-g-flag",
    "no-useless-character-class",
    "no-empty-string-literal",
    "no-optional-assertion",
    "require-unicode-sets-regexp",
    "confusing-quantifier",
    "prefer-named-replacement",
];

pub fn implemented_regexp_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_regexp(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 16]> {
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
    };
    scanner.scan_program(&parser_return.program.body);
    scanner.diagnostics
}
