//! Rule `no-redundant-parentheses` (SonarJS key S1110).
//!
//! Clean-room port. Flags parentheses that are unnecessary because they add no
//! grouping over what an already-parenthesized inner expression provides.
//!
//! To guarantee **zero false positives** under the clean-room constraint, this
//! port intentionally implements ONLY the unambiguous *nested
//! double-parenthesis* subset: a `ParenthesizedExpression` whose direct inner
//! child is itself a `ParenthesizedExpression`, i.e. `((x))`. The inner pair is
//! always redundant under every possible interpretation, regardless of operator
//! precedence. Single-pair redundancy (e.g. `(a + b) * c` vs `(a) + b`) requires
//! operator-precedence reasoning and is FP-prone, so it is deliberately
//! NOT attempted — this rule under-reports by design.
//!
//! Detection relies on the parser running with `preserve_parens: true`, so
//! parentheses are retained in the AST as
//! `Expression::ParenthesizedExpression`. For each parenthesized expression we
//! inspect its DIRECT `.expression` child (NOT `get_inner_expression`, which
//! would strip parens): if that child is itself a `ParenthesizedExpression`, the
//! inner pair is redundant and is reported at the inner pair's span.
//!
//! Because every `ParenthesizedExpression` is visited, triple nesting reports
//! once per redundant inner pair: `(((x)))` yields TWO reports (the middle and
//! the innermost pair are each the direct child of an outer pair).
//!
//! Behaviour is reproduced from the public SonarSource rule documentation
//! (S1110) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! ## Flagged
//! - `((x))` — the inner pair is redundant
//! - `const y = ((1 + 2));` — the inner pair is redundant
//! - `(((x)))` — two reports, one per redundant inner pair
//!
//! ## Not flagged
//! - `(x)` — a single pair carries no nested redundancy
//! - `(a + b) * c` — single pair, precedence-significant
//! - `f((a), (b))` — each argument is a single pair

use oxc_ast::ast::{Expression, ParenthesizedExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-redundant-parentheses";

impl<'a> Scanner<'a> {
    /// Reports the inner pair of a directly nested double parenthesis `((x))`.
    /// Only the unambiguous nested-pair subset is handled; single-pair
    /// redundancy is intentionally not considered.
    pub(crate) fn check_redundant_parentheses(&mut self, it: &ParenthesizedExpression<'a>) {
        if let Expression::ParenthesizedExpression(inner) = &it.expression {
            self.report(RULE_NAME, "redundantParentheses", inner.span);
        }
    }
}
