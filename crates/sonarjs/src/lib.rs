#![doc = "Rust implementation of eslint-plugin-sonarjs rule logic (clean-room port)."]
//!
//! Upstream `eslint-plugin-sonarjs` is LGPL-3.0. Every rule here is implemented
//! clean-room from the public RSPEC documentation and observed behaviour only;
//! no upstream source, tests, fixtures, helper code, or messages are copied.

mod regex_ast;
mod rules;
mod scanner;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;
pub(crate) use crate::types::LineIndex;
pub use crate::types::{Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, SonarjsOptions};

/// Names of every rule implemented by the sonarjs core, in registration order.
pub const RULE_NAMES: [&str; 170] = [
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
    "label-position",
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
    "no-useless-increment",
    "class-name",
    "function-name",
    "max-lines",
    "nested-control-flow",
    "max-lines-per-function",
    "no-duplicate-string",
    "no-empty-group",
    "no-empty-alternatives",
    "no-regex-spaces",
    "no-control-regex",
    "single-char-in-character-classes",
    "duplicates-in-character-class",
    "anchor-precedence",
    "cyclomatic-complexity",
    "no-collection-size-mischeck",
    "index-of-compare-to-positive-number",
    "no-nested-functions",
    "too-many-break-or-continue-in-loop",
    "code-eval",
    "void-use",
    "prefer-promise-shorthand",
    "pseudo-random",
    "hashing",
    "no-clear-text-protocols",
    "no-hardcoded-ip",
    "no-global-this",
    "single-character-alternation",
    "empty-string-repetition",
    "no-misleading-array-reverse",
    "no-alphabetical-sort",
    "no-for-in-iterable",
    "no-associative-arrays",
    "bitwise-operators",
    "no-same-argument-assert",
    "inverted-assertion-arguments",
    "for-loop-increment-sign",
    "no-equals-in-for-termination",
    "reduce-initial-value",
    "no-parameter-reassignment",
    "array-callback-without-return",
    "declarations-in-global-scope",
    "no-wildcard-import",
    "updated-loop-counter",
    "misplaced-loop-counter",
    "no-array-delete",
    "no-literal-call",
    "shorthand-property-grouping",
    "process-argv",
    "standard-input",
    "no-code-after-done",
    "function-inside-loop",
    "no-useless-intersection",
    "use-type-alias",
    "public-static-readonly",
    "call-argument-line",
    "prefer-object-literal",
    "no-undefined-argument",
    "no-identical-functions",
    "no-in-misuse",
    "no-require-or-define",
    "no-invalid-regexp",
    "no-invariant-returns",
    "no-extra-arguments",
    "link-with-target-blank",
    "no-weak-cipher",
    "no-hardcoded-passwords",
    "no-ignored-exceptions",
    "no-unused-function-argument",
    "object-alt-content",
    "no-use-of-empty-return-value",
    "no-duplicated-branches",
    "block-scoped-var",
    "no-variable-usage-before-declaration",
    "arguments-order",
    "updated-const-var",
    "unicode-aware-regex",
    "no-undefined-assignment",
    "no-empty-after-reluctant",
    "no-ignored-return",
    "file-name-differ-from-class",
    "no-unenclosed-multiline-block",
    "inconsistent-function-call",
    "new-operator-misuse",
    "no-empty-test-file",
    "deprecation",
    "cognitive-complexity",
    "expression-complexity",
    "prefer-regexp-exec",
    "no-fallthrough",
    "no-commented-code",
    "no-incomplete-assertions",
    "destructuring-assignment-syntax",
    "no-element-overwrite",
    "no-redundant-assignments",
    "no-unused-collection",
    "no-empty-collection",
    "no-redundant-parentheses",
    "bool-param-default",
    "post-message",
    "in-operator-type-error",
    "different-types-comparison",
    "operation-returning-nan",
    "production-debug",
    "no-hardcoded-secrets",
    "concise-regex",
    "no-misleading-character-class",
    "slow-regex",
    "web-sql-database",
    "no-intrusive-permissions",
    "encryption-secure-mode",
    "no-unsafe-unzip",
    "disabled-timeout",
    "cookie-no-httponly",
    "content-security-policy",
    "certificate-transparency",
    "csrf",
    "file-permissions",
    "file-uploads",
    "cors",
    "dns-prefetching",
    "disabled-auto-escaping",
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

    // Semantic analysis resolves identifier references and declaration sites,
    // which `no-misleading-array-reverse`, `no-alphabetical-sort`,
    // `no-for-in-iterable`, and `no-associative-arrays` need (to prove that an
    // identifier refers to an array). Build it only when one of those rules is
    // active so the other
    // rules don't pay for an extra AST walk. Benign semantic errors (e.g.
    // redeclarations) do not block scanning.
    let needs_semantic = options.has_rule("no-misleading-array-reverse")
        || options.has_rule("no-alphabetical-sort")
        || options.has_rule("no-for-in-iterable")
        || options.has_rule("no-associative-arrays")
        || options.has_rule("reduce-initial-value")
        || options.has_rule("no-parameter-reassignment")
        || options.has_rule("updated-loop-counter")
        || options.has_rule("no-array-delete")
        || options.has_rule("no-in-misuse")
        || options.has_rule("no-extra-arguments")
        || options.has_rule("no-unused-function-argument")
        || options.has_rule("no-use-of-empty-return-value")
        || options.has_rule("block-scoped-var")
        || options.has_rule("no-variable-usage-before-declaration")
        || options.has_rule("arguments-order")
        || options.has_rule("updated-const-var")
        || options.has_rule("inconsistent-function-call")
        || options.has_rule("new-operator-misuse")
        || options.has_rule("deprecation")
        || options.has_rule("no-unused-collection")
        || options.has_rule("no-empty-collection");
    let semantic = needs_semantic.then(|| {
        SemanticBuilder::new()
            .build(&parser_return.program)
            .semantic
    });
    let scoping = semantic.as_ref().map(|semantic| semantic.scoping());
    let nodes = semantic.as_ref().map(|semantic| semantic.nodes());

    let mut scanner = Scanner {
        source_text,
        filename,
        line_index: LineIndex::new(source_text),
        options,
        diagnostics: SmallVec::new(),
        scoping,
        nodes,
        template_literal_depth: 0,
        switch_depth: 0,
        conditional_depth: 0,
        if_chain_seen: SmallVec::new(),
        generator_yield_stack: SmallVec::new(),
        return_kind_stack: SmallVec::new(),
        invariant_return_stack: SmallVec::new(),
        control_flow_depth: 0,
        else_if_starts: SmallVec::new(),
        comment_spans: SmallVec::new(),
        jsx_function_stack: SmallVec::new(),
        iife_function_starts: SmallVec::new(),
        seen_function_impls: SmallVec::new(),
        string_literals: SmallVec::new(),
        excluded_string_starts: SmallVec::new(),
        composite_types: SmallVec::new(),
        cyclomatic_complexity_stack: SmallVec::new(),
        expression_complexity_stack: SmallVec::new(),
        function_nesting_depth: 0,
        this_binding_depth: 0,
        breakable_stack: SmallVec::new(),
        pending_loop_label: None,
        loop_counter_symbols: SmallVec::new(),
        loop_depth_in_function: core::iter::once(0u32).collect(),
        fn_span_stack: SmallVec::new(),
        fn_call_new_records: SmallVec::new(),
        saw_test_call: false,
        deprecated_symbols: SmallVec::new(),
        cognitive_complexity_fn_depth: 0,
    };
    scanner.visit_program(&parser_return.program);
    scanner.diagnostics
}
