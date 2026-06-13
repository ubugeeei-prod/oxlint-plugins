//! Rule `no-redundant-boolean` (SonarJS key S1125).
//!
//! Clean-room port. Reports boolean literals that serve no purpose because the
//! surrounding expression already conveys the same intent without them. Three
//! patterns are flagged:
//!
//! 1. **Equality comparison with a boolean literal** — a `BinaryExpression`
//!    whose operator is `==`, `===`, `!=`, or `!==` where either operand is a
//!    boolean literal. The span reported is the boolean literal itself (left
//!    preferred when both sides are boolean literals).
//! 2. **Negation of a boolean literal** — a `UnaryExpression` with operator
//!    `!` whose argument is a boolean literal. The whole unary expression is
//!    reported.
//! 3. **Ternary returning only boolean literals** — a `ConditionalExpression`
//!    whose `consequent` AND `alternate` are both boolean literals (covers all
//!    four combinations of `true`/`false`). The whole conditional expression is
//!    reported.
//!
//! **Out of scope**: `&&` and `||` with boolean literals are intentionally NOT
//! flagged by this rule. Those patterns belong to a separate rule and are
//! excluded here to avoid false positives in common idioms such as
//! `flag && doSomething()`.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{BinaryExpression, ConditionalExpression, Expression, UnaryExpression};
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-redundant-boolean";

/// Returns `true` when `expr` (after stripping parentheses) is a boolean literal.
fn is_boolean_literal(expr: &Expression<'_>) -> bool {
    matches!(expr.get_inner_expression(), Expression::BooleanLiteral(_))
}

impl Scanner<'_> {
    pub(crate) fn check_no_redundant_boolean_binary(&mut self, expr: &BinaryExpression<'_>) {
        let is_eq_op = matches!(
            expr.operator,
            BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
        );
        if !is_eq_op {
            return;
        }
        let left_is_bool = is_boolean_literal(&expr.left);
        let right_is_bool = is_boolean_literal(&expr.right);
        if !left_is_bool && !right_is_bool {
            return;
        }
        // Report the boolean literal's span; prefer left when both are boolean literals.
        let span = if left_is_bool {
            expr.left.span()
        } else {
            expr.right.span()
        };
        self.report(RULE_NAME, "redundantBoolean", span);
    }

    pub(crate) fn check_no_redundant_boolean_unary(&mut self, expr: &UnaryExpression<'_>) {
        if expr.operator != UnaryOperator::LogicalNot {
            return;
        }
        if !is_boolean_literal(&expr.argument) {
            return;
        }
        self.report(RULE_NAME, "redundantBoolean", expr.span());
    }

    pub(crate) fn check_no_redundant_boolean_conditional(
        &mut self,
        expr: &ConditionalExpression<'_>,
    ) {
        if !is_boolean_literal(&expr.consequent) || !is_boolean_literal(&expr.alternate) {
            return;
        }
        self.report(RULE_NAME, "redundantBoolean", expr.span());
    }
}
