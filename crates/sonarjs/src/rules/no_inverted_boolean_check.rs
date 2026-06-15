//! Rule `no-inverted-boolean-check` (SonarJS key S1940).
//!
//! Clean-room port. Reports a logical-NOT applied directly to a comparison
//! expression. Negating a comparison makes the intent harder to read; the
//! opposite comparison operator should be used instead. For example,
//! `!(a === b)` is clearer as `a !== b`, and `!(a < b)` is clearer as `a >= b`.
//!
//! ## Covered comparison operators
//!
//! The following `BinaryExpression` operators are flagged when negated:
//! - Equality: `==` (`Equality`), `===` (`StrictEquality`)
//! - Inequality: `!=` (`Inequality`), `!==` (`StrictInequality`)
//! - Ordering: `<` (`LessThan`), `<=` (`LessEqualThan`), `>` (`GreaterThan`),
//!   `>=` (`GreaterEqualThan`)
//!
//! ## Explicitly excluded
//!
//! - Logical operators `&&` and `||` — these produce a `LogicalExpression`
//!   (not a `BinaryExpression`) and are therefore naturally excluded.
//! - Arithmetic binary operators such as `+`, `-`, `*`, `/` — these are
//!   `BinaryExpression` nodes but their operators do not appear in the targeted
//!   set, so they are filtered out by the `matches!` guard.
//! - Plain negation of a non-comparison: `!a`, `!foo()` — the argument is not
//!   a `BinaryExpression` at all, so the early-return fires.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Expression, UnaryExpression};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-inverted-boolean-check";

impl Scanner<'_> {
    pub(crate) fn check_no_inverted_boolean_check(&mut self, expr: &UnaryExpression<'_>) {
        if expr.operator != UnaryOperator::LogicalNot {
            return;
        }
        let Expression::BinaryExpression(binary) = expr.argument.get_inner_expression() else {
            return;
        };
        let is_comparison = matches!(
            binary.operator,
            BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
                | BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan
        );
        if !is_comparison {
            return;
        }
        self.report(RULE_NAME, "invertedBooleanCheck", expr.span);
    }
}
