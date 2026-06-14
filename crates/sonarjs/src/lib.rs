#![doc = "Rust implementation of eslint-plugin-sonarjs rule logic (clean-room port)."]
//!
//! Upstream `eslint-plugin-sonarjs` is LGPL-3.0. Every rule here is implemented
//! clean-room from the public RSPEC documentation and observed behaviour only;
//! no upstream source, tests, fixtures, helper code, or messages are copied.

mod rules;
mod scanner;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;
pub(crate) use crate::types::LineIndex;
pub use crate::types::{Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, SonarjsOptions};

/// Names of every rule implemented by the sonarjs core, in registration order.
pub const RULE_NAMES: [&str; 47] = [
    "no-nested-template-literals",
    "no-nested-switch",
    "no-nested-conditional",
    "no-collapsible-if",
    "no-redundant-boolean",
    "comma-or-logical-or-case",
    "no-duplicate-in-composite",
    "non-existent-operator",
    "no-identical-conditions",
    "no-all-duplicated-branches",
    "no-identical-expressions",
    "arguments-usage",
    "no-labels",
    "no-delete-var",
    "constructor-for-side-effects",
    "no-empty-character-class",
    "generator-without-yield",
    "no-exclusive-tests",
    "no-built-in-override",
    "class-prototype",
    "max-switch-cases",
    "max-union-size",
    "elseif-without-else",
    "no-case-label-in-switch",
    "for-in",
    "prefer-while",
    "no-small-switch",
    "prefer-default-last",
    "no-inverted-boolean-check",
    "no-useless-catch",
    "no-redundant-optional",
    "prefer-immediate-return",
    "no-redundant-jump",
    "no-primitive-wrappers",
    "no-skipped-tests",
    "prefer-single-boolean-return",
    "no-unthrown-error",
    "no-tab",
    "fixme-tag",
    "todo-tag",
    "no-sonar-comments",
    "array-constructor",
    "no-function-declaration-in-block",
    "no-inconsistent-returns",
    "no-same-line-conditional",
    "no-nested-assignment",
    "no-nested-incdec",
];

/// Returns the implemented rule names as a static slice.
pub fn implemented_sonarjs_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

/// Parses `source_text` and returns the diagnostics produced by the rules
/// enabled in `options`. Files that fail to parse produce no diagnostics.
pub fn scan_sonarjs(
    source_text: &str,
    filename: &str,
    options: &SonarjsOptions,
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
        options,
        diagnostics: SmallVec::new(),
        template_literal_depth: 0,
        switch_depth: 0,
        conditional_depth: 0,
        if_chain_seen: SmallVec::new(),
        generator_yield_stack: SmallVec::new(),
        return_kind_stack: SmallVec::new(),
    };
    scanner.visit_program(&parser_return.program);
    scanner.diagnostics
}
