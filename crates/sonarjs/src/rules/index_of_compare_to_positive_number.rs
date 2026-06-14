//! Rule `index-of-compare-to-positive-number` (SonarJS key S2692).
//!
//! Clean-room port. Reports a `BinaryExpression` where one operand (after
//! stripping parentheses) is a `CallExpression` whose callee is a
//! `StaticMemberExpression` with property name `indexOf` or `lastIndexOf`,
//! and the other operand is a numeric literal, with the comparison silently
//! excluding the element at index 0:
//!
//! - `<call> > N`  where N >= 0   (bug: misses indices 0 through N)
//! - `<call> >= N` where N >= 1   (bug: misses the element at index 0)
//! - `N < <call>`  where N >= 0   (mirrored form of `> N`)
//! - `N <= <call>` where N >= 1   (mirrored form of `>= N`)
//!
//! Compliant comparisons (`>= 0`, `> -1`, `=== -1`, `!== -1`, `< 0`,
//! `<= -1`) are NOT flagged.
//!
//! Detection is purely syntactic: the rule fires on any call whose property
//! name is `indexOf` or `lastIndexOf` regardless of the receiver's type.
//! This matches the behaviour of `eslint-plugin-sonarjs` when running without
//! type information.
//!
//! Behaviour is reproduced from the public RSPEC S2692 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "index-of-compare-to-positive-number";

/// Returns `true` when `expr` (after stripping parentheses) is a call whose
/// callee is a static member expression with property `indexOf` or
/// `lastIndexOf`.
fn is_index_of_call(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::CallExpression(call) => match call.callee.get_inner_expression() {
            Expression::StaticMemberExpression(m) => {
                m.property.name == "indexOf" || m.property.name == "lastIndexOf"
            }
            _ => false,
        },
        _ => false,
    }
}

/// Returns `true` when `expr` (after stripping parentheses) is a numeric
/// literal with value >= `min`.
fn is_num_gte(expr: &Expression<'_>, min: f64) -> bool {
    match expr.get_inner_expression() {
        Expression::NumericLiteral(n) => n.value >= min,
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_index_of_compare_to_positive_number(
        &mut self,
        expr: &BinaryExpression<'_>,
    ) {
        // Shape 1: <indexOf> > N  where N >= 0   (bug: misses index 0..N)
        // Shape 2: <indexOf> >= N where N >= 1   (bug: misses index 0)
        // Shape 3: N < <indexOf>  where N >= 0   (mirror of shape 1)
        // Shape 4: N <= <indexOf> where N >= 1   (mirror of shape 2)
        let flagged = matches!(expr.operator, BinaryOperator::GreaterThan)
            && is_index_of_call(&expr.left)
            && is_num_gte(&expr.right, 0.0)
            || matches!(expr.operator, BinaryOperator::GreaterEqualThan)
                && is_index_of_call(&expr.left)
                && is_num_gte(&expr.right, 1.0)
            || matches!(expr.operator, BinaryOperator::LessThan)
                && is_num_gte(&expr.left, 0.0)
                && is_index_of_call(&expr.right)
            || matches!(expr.operator, BinaryOperator::LessEqualThan)
                && is_num_gte(&expr.left, 1.0)
                && is_index_of_call(&expr.right);
        if flagged {
            self.report(RULE_NAME, "indexOfPositive", expr.span);
        }
    }
}
