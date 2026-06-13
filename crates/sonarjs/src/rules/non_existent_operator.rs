//! Rule `non-existent-operator` (SonarJS key S2757).
//!
//! Clean-room port. Detects a likely typo where the programmer wrote `=-`, `=+`,
//! or `=!` by placing a plain assignment (`=`) immediately adjacent (no whitespace)
//! to a unary `-`, `+`, or `!` operator. The code parses validly — `x =- 1` is
//! `x = (-1)` — but the visual resemblance to the compound operators `-=`, `+=`,
//! and the comparison `!=` makes it a frequent source of bugs.
//!
//! **Detection heuristic**: The rule triggers when ALL of the following hold:
//!
//! 1. The expression is an `AssignmentExpression` with the plain `=` operator.
//! 2. The right-hand side (after stripping parentheses) is a `UnaryExpression`
//!    with operator `-`, `+`, or `!`.
//! 3. The source byte immediately *before* the unary operator is `=` (i.e. no
//!    whitespace between the `=` and the unary symbol).
//!
//! Condition 3 is what distinguishes the suspicious `x =- 1` from the intentional
//! `x = -1` (which has a space and is NOT flagged).
//!
//! The entire `AssignmentExpression` span is reported.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{AssignmentExpression, Expression};
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "non-existent-operator";

impl Scanner<'_> {
    pub(crate) fn check_non_existent_operator(&mut self, assign: &AssignmentExpression<'_>) {
        if assign.operator != AssignmentOperator::Assign {
            return;
        }
        let Expression::UnaryExpression(unary) = assign.right.get_inner_expression() else {
            return;
        };
        let is_target_op = matches!(
            unary.operator,
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::LogicalNot
        );
        if !is_target_op {
            return;
        }
        // The unary expression's span starts at the unary operator character (e.g. `-`).
        // If the byte immediately before it is `=`, the two tokens are adjacent — the
        // suspicious `=-` / `=+` / `=!` pattern. A space between them means the
        // assignment is intentional and should not be flagged.
        let start = unary.span.start as usize;
        let adjacent = start >= 1 && self.source_text.as_bytes().get(start - 1) == Some(&b'=');
        if !adjacent {
            return;
        }
        self.report(RULE_NAME, "nonExistentOperator", assign.span());
    }
}
