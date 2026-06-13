//! Rule `no-nested-conditional` (SonarJS key S3358).
//!
//! Clean-room port. Reports a conditional (ternary) expression that appears
//! nested inside another conditional expression, in any of the outer ternary's
//! three parts (test, consequent, or alternate). Nested ternaries hurt
//! readability and make control flow difficult to follow at a glance.
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Semantics: a conditional expression is flagged when at least one enclosing
//! conditional expression is open on the traversal stack, so every nested
//! ternary is reported at its own span regardless of nesting depth.

use oxc_ast::ast::ConditionalExpression;
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-nested-conditional";

impl Scanner<'_> {
    pub(crate) fn check_no_nested_conditional(&mut self, expr: &ConditionalExpression<'_>) {
        if self.conditional_depth > 0 {
            self.report(RULE_NAME, "nestedConditional", expr.span());
        }
    }
}
