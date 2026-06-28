//! Rule `arrow-function-convention` (SonarJS key S3524).
//!
//! Clean-room port. SonarJS S3524 ("Braces and parentheses should be used
//! consistently with arrow functions") enforces a single, consistent style for
//! the optional syntax around arrow functions: the parentheses around a single
//! parameter, and the curly braces around a single-expression body.
//!
//! ## Narrow form
//!
//! The upstream rule is configurable through two boolean options
//! (`parameter_parentheses` and `body_braces`) that switch each check between an
//! "always require" and an "as-needed" style. This port implements only the
//! **default** SonarJS configuration, in which both options are `false`, i.e.
//! the *as-needed* style. It reports the two unambiguous, removable cases:
//!
//! ```js
//! const f = (x) => x;          // Noncompliant: drop the parentheses -> x => x
//! const g = x => { return x; } // Noncompliant: drop the braces -> x => x
//! ```
//!
//! and accepts the already-minimal forms:
//!
//! ```js
//! const f = x => x;            // Compliant
//! const g = (a, b) => a + b;   // Compliant (parentheses are mandatory)
//! ```
//!
//! ### Zero-false-positive guards
//!
//! Parentheses are flagged for removal ONLY when the arrow has exactly one
//! plain binding-identifier parameter with no type annotation, no default
//! value, no decorator, and is neither optional nor a rest element, and the
//! arrow carries neither type parameters nor an explicit return type (any of
//! these make the parentheses syntactically mandatory). Presence of the
//! parentheses is confirmed against the source text, so destructuring,
//! multi-parameter, and zero-parameter arrows are never touched.
//!
//! Braces are flagged for removal ONLY when a block body consists of exactly
//! one `return <expr>;` statement with an argument, has no directive prologue,
//! and contains no comment between the braces (removing the braces would discard
//! such a comment). Bodies with bare `return;`, multiple statements, or other
//! statement kinds are left untouched.
//!
//! The "always require" variants of both options are a documented follow-up and
//! are intentionally not implemented, so this port only ever under-reports.
//!
//! Behaviour is reproduced from the public RSPEC description (S3524) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{ArrowFunctionExpression, BindingPattern, Statement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "arrow-function-convention";

impl Scanner<'_> {
    pub(crate) fn check_arrow_function_convention(&mut self, arrow: &ArrowFunctionExpression<'_>) {
        self.check_arrow_parameter_parentheses(arrow);
        self.check_arrow_body_braces(arrow);
    }

    /// Flags removable parentheses around a single plain-identifier parameter.
    fn check_arrow_parameter_parentheses(&mut self, arrow: &ArrowFunctionExpression<'_>) {
        // Type parameters (`<T>(x) => x`) or an explicit return type
        // (`(x): T => x`) make the parentheses mandatory.
        if arrow.type_parameters.is_some() || arrow.return_type.is_some() {
            return;
        }
        // Only a single, non-rest parameter can shed its parentheses.
        if arrow.params.rest.is_some() || arrow.params.items.len() != 1 {
            return;
        }
        let param = &arrow.params.items[0];
        if !param.decorators.is_empty()
            || param.type_annotation.is_some()
            || param.initializer.is_some()
            || param.optional
        {
            return;
        }
        // Destructuring (`({ x }) => ...`, `([x]) => ...`) requires parentheses.
        if !matches!(param.pattern, BindingPattern::BindingIdentifier(_)) {
            return;
        }
        // Confirm the parentheses are actually present before reporting.
        if !self.has_open_paren_before(arrow.span.start, param.span.start) {
            return;
        }
        self.report(RULE_NAME, "removeParens", arrow.params.span);
    }

    /// Scans backwards from `from` (exclusive) down to `lo` over ASCII
    /// whitespace and returns `true` when the first non-whitespace byte is an
    /// opening parenthesis. Used to detect that an arrow's single parameter is
    /// parenthesised without relying on the exact `FormalParameters` span.
    fn has_open_paren_before(&self, lo: u32, from: u32) -> bool {
        let bytes = self.source_text.as_bytes();
        let lo = lo as usize;
        let mut i = from as usize;
        while i > lo {
            i -= 1;
            let b = bytes[i];
            if b == b'(' {
                return true;
            }
            if !b.is_ascii_whitespace() {
                return false;
            }
        }
        false
    }

    /// Flags a block body that is a single `return <expr>;` statement.
    fn check_arrow_body_braces(&mut self, arrow: &ArrowFunctionExpression<'_>) {
        if arrow.expression {
            return;
        }
        let body = &arrow.body;
        if !body.directives.is_empty() || body.statements.len() != 1 {
            return;
        }
        let Statement::ReturnStatement(ret) = &body.statements[0] else {
            return;
        };
        if ret.argument.is_none() {
            return;
        }
        let body_span = body.span;
        // Removing the braces would discard any comment that lives inside them.
        let has_inner_comment = self
            .comment_spans
            .iter()
            .any(|c| c.start >= body_span.start && c.end <= body_span.end);
        if has_inner_comment {
            return;
        }
        self.report(RULE_NAME, "removeBraces", body_span);
    }
}
