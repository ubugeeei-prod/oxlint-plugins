#![doc = "Rust implementation of eslint-plugin-simple-import-sort rule logic."]

mod shared;
mod scanner;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_ast::ast::Comment;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;

use crate::types::LineIndex;

pub use crate::types::{Diagnostic, DiagnosticFix, DiagnosticLoc, SimpleImportSortOptions};

pub const RULE_NAMES: [&str; 2] = ["exports", "imports"];

pub fn implemented_simple_import_sort_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_simple_import_sort(
    source_text: &str,
    filename: &str,
    options: &SimpleImportSortOptions,
) -> SmallVec<[Diagnostic; 8]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    // Collect all comments in source order (OXC guarantees sorted order)
    let all_comments: SmallVec<[Comment; 32]> = parser_return
        .program
        .comments
        .iter()
        .copied()
        .collect();

    let line_index = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();

    scanner::scan_import_chunks(
        source_text,
        &line_index,
        &parser_return.program.body,
        &all_comments,
        options,
        &mut diagnostics,
    );
    scanner::scan_export_chunks(
        source_text,
        &line_index,
        &parser_return.program.body,
        &all_comments,
        &mut diagnostics,
    );
    diagnostics
}
