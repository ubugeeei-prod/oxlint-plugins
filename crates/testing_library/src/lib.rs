#![doc = "Rust implementation of eslint-plugin-testing-library rule logic."]
#![allow(
    clippy::disallowed_types,
    reason = "The native scanner stores compact NAPI-facing diagnostics and scans short source slices."
)]

mod checks;
mod helpers;
mod scanner;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;
use crate::types::LineIndex;

pub use crate::types::{Diagnostic, DiagnosticLoc, TestingLibraryOptions};

pub const RULE_NAMES: [&str; 29] = [
    "await-async-events",
    "await-async-queries",
    "await-async-utils",
    "consistent-data-testid",
    "no-await-sync-events",
    "no-await-sync-queries",
    "no-container",
    "no-debugging-utils",
    "no-dom-import",
    "no-global-regexp-flag-in-query",
    "no-manual-cleanup",
    "no-node-access",
    "no-promise-in-fire-event",
    "no-render-in-lifecycle",
    "no-test-id-queries",
    "no-unnecessary-act",
    "no-wait-for-multiple-assertions",
    "no-wait-for-side-effects",
    "no-wait-for-snapshot",
    "prefer-explicit-assert",
    "prefer-find-by",
    "prefer-implicit-assert",
    "prefer-presence-queries",
    "prefer-query-by-disappearance",
    "prefer-query-matchers",
    "prefer-screen-queries",
    "prefer-user-event",
    "prefer-user-event-setup",
    "render-result-naming-convention",
];

pub fn implemented_testing_library_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_testing_library(
    source_text: &str,
    filename: &str,
    options: &TestingLibraryOptions,
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
        filename,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        options,
    };
    scanner.scan();
    scanner.diagnostics
}
