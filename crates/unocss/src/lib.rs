#![doc = "Rust implementation of @unocss/eslint-plugin rule logic."]

mod ordering;
mod scanner;
mod types;
mod visitor;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;
use regex::RegexBuilder;

use crate::scanner::Scanner;
use crate::types::LineIndex;

pub use crate::types::{BlocklistEntry, Diagnostic, DiagnosticFix, DiagnosticLoc, UnocssOptions};

pub const RULE_NAMES: [&str; 4] = [
    "blocklist",
    "enforce-class-compile",
    "order",
    "order-attributify",
];

pub fn implemented_unocss_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_unocss(
    source_text: &str,
    filename: &str,
    options: &UnocssOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    // Build case-insensitive regexes matching upstream `new RegExp(regex, 'i')`.
    let variable_regexes = options
        .uno_variables
        .iter()
        .filter_map(|pattern| {
            RegexBuilder::new(pattern.as_str())
                .case_insensitive(true)
                .build()
                .ok()
        })
        .collect();

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        options: options.clone(),
        variable_regexes,
        diagnostics: SmallVec::new(),
    };
    scanner.run(&parser_return.program);
    scanner.diagnostics
}
