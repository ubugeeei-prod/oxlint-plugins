//! Rule `no-commented-code` (SonarJS key S125).
//!
//! Clean-room port. Flags comments whose inner text is parseable as valid
//! JavaScript/TypeScript and exhibits a strong code signal. Prose-only
//! comments are never flagged.
//!
//! ## Heuristic (conservative, zero-false-positive bias)
//!
//! For each comment in the file:
//! 1. Skip JSDoc block comments (those whose raw text starts with `/**`).
//! 2. Skip comments whose inner text contains a well-known special tag:
//!    `eslint-disable`, `@ts-`, `NOSONAR`, `TODO`, or `FIXME`.
//! 3. Skip comments whose trimmed inner text is shorter than 5 characters.
//! 4. Extract the inner text by stripping `//` or `/* */` delimiters using
//!    `strip_prefix`/`strip_suffix` (never manual index arithmetic).
//! 5. Parse the trimmed inner text as TSX/module source with a fresh
//!    `Allocator` (one per comment; no state shared between iterations).
//! 6. Flag the comment only when **all** of the following hold:
//!    - Parsing succeeds with no syntax errors.
//!    - The parsed program contains at least one top-level statement.
//!    - A **strong code signal** is present (see below).
//!
//! **Strong code signal** — at least one of:
//! - The trimmed inner text ends with `;`, `{`, `}`, or `,`.
//! - The program has exactly one top-level statement and that statement is
//!   an unambiguously code-only construct: `if`, `for`, `while`, `do-while`,
//!   `for-in`, `for-of`, `switch`, `try`, `throw`, a function declaration,
//!   a class declaration, or a variable declaration.
//!
//! Expression statements (bare identifiers, arithmetic, labelled expressions)
//! are NOT strong code signals on their own, so prose that happens to parse
//! as valid JS (e.g. `a + b` or `name: value`) is **not** reported.
//!
//! Multi-line block comments that use a leading ` * ` prefix on each line
//! (the conventional JSDoc/JavaDoc style) fail to parse as JS (because `*`
//! at the start of a statement is a syntax error without a left-hand side),
//! and are therefore never flagged by the parse step.
//!
//! Behaviour derived from the public SonarJS RSPEC description (S125) only;
//! no upstream source, tests, fixtures, or message strings were copied.

use oxc_allocator::Allocator;
use oxc_ast::ast::{Comment, Statement};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-commented-code";

/// Strips comment delimiters from a raw comment string and returns the inner
/// text, or `None` when the format is unrecognised.
///
/// - Line comments (`//…`): returns everything after the `//` prefix.
/// - Block comments (`/* … */`): strips the opening `/*` and closing `*/`.
/// - Any other format (e.g. JSDoc `/**`, already excluded upstream): `None`.
///
/// Uses `str::strip_prefix` / `str::strip_suffix` to avoid manual index
/// arithmetic (avoids the `clippy::manual_strip` lint).
fn strip_comment_delimiters(full_text: &str) -> Option<&str> {
    // Line comment: everything after "//".
    if let Some(inner) = full_text.strip_prefix("//") {
        return Some(inner);
    }
    // Block comment: strip trailing "*/" first, then the opening "/*".
    if let Some(without_close) = full_text.strip_suffix("*/") {
        return without_close.strip_prefix("/*");
    }
    None
}

/// Returns `true` when the inner text of a comment contains a well-known tag
/// that marks it as non-code (a lint directive, type suppressor, or an
/// in-progress work marker handled by a dedicated rule).
fn is_special_comment(inner: &str) -> bool {
    let trimmed = inner.trim_start();
    if trimmed.starts_with("TODO") || trimmed.starts_with("FIXME") {
        return true;
    }
    inner.contains("eslint-disable") || inner.contains("@ts-") || inner.contains("NOSONAR")
}

/// Returns `true` when a single top-level statement is of a type that prose
/// text could not plausibly produce and that requires intentional JS authoring.
fn is_strong_statement(stmt: &Statement<'_>) -> bool {
    matches!(
        stmt,
        Statement::IfStatement(_)
            | Statement::ForStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::SwitchStatement(_)
            | Statement::TryStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::FunctionDeclaration(_)
            | Statement::ClassDeclaration(_)
            | Statement::VariableDeclaration(_)
    )
}

/// Returns `true` when the parsed comment content exhibits at least one strong
/// code signal — either a code-specific terminal character or a structurally
/// unambiguous single-statement program.
fn has_strong_code_signal(trimmed: &str, stmts: &[Statement<'_>]) -> bool {
    // Terminal-character signal: these characters rarely end natural-language
    // prose but frequently end JavaScript statements and expressions.
    if trimmed.ends_with(';')
        || trimmed.ends_with('{')
        || trimmed.ends_with('}')
        || trimmed.ends_with(',')
    {
        return true;
    }
    // Statement-type signal: a single top-level statement whose type is
    // unambiguously code (control flow or declaration).
    if stmts.len() == 1 {
        return is_strong_statement(&stmts[0]);
    }
    false
}

impl Scanner<'_> {
    pub(crate) fn check_no_commented_code(&mut self, comments: &[Comment]) {
        let mut spans: SmallVec<[Span; 8]> = SmallVec::new();
        for comment in comments {
            let full_text = self.text(comment.span);

            // Skip JSDoc block comments (/** … */) — these are documentation,
            // not commented-out code.
            if full_text.starts_with("/**") {
                continue;
            }

            // Strip the comment delimiters to obtain the raw inner text.
            let Some(inner) = strip_comment_delimiters(full_text) else {
                continue;
            };

            // Skip comments that carry well-known special-purpose tags.
            if is_special_comment(inner) {
                continue;
            }

            let trimmed = inner.trim();

            // Skip trivially short content — too little text to make a
            // confident code-vs-prose decision (false positives unacceptable).
            if trimmed.len() < 5 {
                continue;
            }

            // Parse the comment body with a fresh arena allocator.  A new
            // allocator is created for every comment so that the borrow of
            // the parsed program (which references the arena) never escapes
            // this loop iteration.
            let allocator = Allocator::default();
            let source_type = SourceType::tsx().with_module(true);
            let parse_result = Parser::new(&allocator, trimmed, source_type).parse();

            // Any syntax error means the content is NOT valid JS → skip.
            if !parse_result.errors.is_empty() {
                continue;
            }
            // An empty program (e.g. a comment-only inner string) is not code.
            if parse_result.program.body.is_empty() {
                continue;
            }
            // Require a strong code signal to suppress prose false positives.
            if !has_strong_code_signal(trimmed, &parse_result.program.body) {
                continue;
            }

            spans.push(comment.span);
        }
        for span in spans {
            self.report(RULE_NAME, "commentedCode", span);
        }
    }
}
