#![doc = "Rust implementation of @unocss/eslint-plugin rule logic."]

mod literals;
mod ordering;
mod scanner;
mod tags;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;
use regex::Regex;

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

    let variable_regexes = options
        .uno_variables
        .iter()
        .filter_map(|pattern| Regex::new(pattern.as_str()).ok())
        .collect();
    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        options: options.clone(),
        variable_regexes,
        diagnostics: SmallVec::new(),
    };
    scanner.scan_literals();
    scanner.scan_attributify();
    scanner.diagnostics
}
