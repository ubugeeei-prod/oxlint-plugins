#![doc = "Rust implementation of eslint-plugin-playwright rule logic."]

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

pub use crate::types::{Diagnostic, DiagnosticLoc};

pub const RULE_NAMES: [&str; 58] = [
    "consistent-spacing-between-blocks",
    "expect-expect",
    "max-expects",
    "max-nested-describe",
    "missing-playwright-await",
    "no-commented-out-tests",
    "no-conditional-expect",
    "no-conditional-in-test",
    "no-duplicate-hooks",
    "no-duplicate-slow",
    "no-element-handle",
    "no-eval",
    "no-focused-test",
    "no-force-option",
    "no-get-by-title",
    "no-hooks",
    "no-nested-step",
    "no-networkidle",
    "no-nth-methods",
    "no-page-pause",
    "no-raw-locators",
    "no-restricted-locators",
    "no-restricted-matchers",
    "no-restricted-roles",
    "no-skipped-test",
    "no-slowed-test",
    "no-standalone-expect",
    "no-unsafe-references",
    "no-unused-locators",
    "no-useless-await",
    "no-useless-not",
    "no-wait-for-navigation",
    "no-wait-for-selector",
    "no-wait-for-timeout",
    "prefer-comparison-matcher",
    "prefer-equality-matcher",
    "prefer-hooks-in-order",
    "prefer-hooks-on-top",
    "prefer-locator",
    "prefer-lowercase-title",
    "prefer-native-locators",
    "prefer-strict-equal",
    "prefer-to-be",
    "prefer-to-contain",
    "prefer-to-have-count",
    "prefer-to-have-length",
    "prefer-web-first-assertions",
    "require-hook",
    "require-soft-assertions",
    "require-tags",
    "require-to-pass-timeout",
    "require-to-throw-message",
    "require-top-level-describe",
    "valid-describe-callback",
    "valid-expect",
    "valid-expect-in-promise",
    "valid-test-tags",
    "valid-title",
];

pub fn implemented_playwright_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_playwright(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 64]> {
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
