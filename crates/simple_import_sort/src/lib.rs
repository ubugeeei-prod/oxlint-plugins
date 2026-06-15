#![doc = "Rust implementation of eslint-plugin-simple-import-sort rule logic."]

mod scanner;
mod shared;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_ast::ast::Comment;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;

use crate::types::LineIndex;

/// Precompiled regex groups built once per `scan_simple_import_sort` call.
/// `[outer][inner]` — `None` means the pattern failed to compile.
type CompiledGroupsOwned = SmallVec<[SmallVec<[Option<regex::Regex>; 4]>; 8]>;

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
    // Do not autofix source that does not parse: a partial AST can have unreliable
    // spans, and running an autofix over broken code risks corrupting it. (Oxc is
    // stricter than upstream's TypeScript parser on a few syntactically-invalid
    // constructs, e.g. `import type Def, { Named }`, which upstream tolerates.)
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    // Collect all comments in source order (OXC guarantees sorted order)
    let all_comments: SmallVec<[Comment; 32]> =
        parser_return.program.comments.iter().copied().collect();

    let line_index = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();

    // Precompile regex patterns for custom import groups once per scan.
    // Each pattern that fails to compile is silently skipped (None), matching
    // the per-call behaviour in import_group before this optimisation.
    let compiled_groups: Option<CompiledGroupsOwned> =
        options.import_groups.as_ref().map(|groups| {
            groups
                .iter()
                .map(|inner| {
                    inner
                        .iter()
                        .map(|pat| regex::Regex::new(pat.as_str()).ok())
                        .collect()
                })
                .collect()
        });

    scanner::scan_import_chunks(
        source_text,
        &line_index,
        &parser_return.program.body,
        &all_comments,
        options,
        compiled_groups.as_deref(),
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
