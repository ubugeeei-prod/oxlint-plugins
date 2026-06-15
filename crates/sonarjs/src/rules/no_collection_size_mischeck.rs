//! Rule `no-collection-size-mischeck` (SonarJS key S3981).
//!
//! Clean-room port. Reports a `BinaryExpression` where one operand is a member
//! access reading the property `length` or `size` (e.g. `arr.length`,
//! `map.size`) and the other operand is the numeric literal `0`, AND the
//! comparison is always-true or always-false because `length`/`size` are
//! non-negative integers:
//!
//! - `<expr>.length < 0`  — always false (flag)
//! - `<expr>.length >= 0` — always true  (flag)
//! - `<expr>.size < 0`    — always false (flag)
//! - `<expr>.size >= 0`   — always true  (flag)
//! - and the mirrored forms `0 > <expr>.length` / `0 <= <expr>.length` etc.
//!
//! Only these four always-true/always-false shapes are flagged. Meaningful
//! comparisons (`<= 0`, `> 0`, `=== 0`, `== 0`, `< 1`, etc.) are NOT flagged.
//!
//! Detection is purely syntactic: the rule fires on any `.length`/`.size`
//! member access without verifying the receiver's type. This matches the
//! behaviour of `eslint-plugin-sonarjs` when running without type information.
//!
//! Behaviour is reproduced from the public RSPEC S3981 description and the
//! public docs of `eslint-plugin-sonarjs/no-collection-size-mischeck` only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-collection-size-mischeck";

/// Returns `true` when `expr` (after stripping parentheses) is a static member
/// access with property name `length` or `size`.
fn is_length_or_size(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::StaticMemberExpression(m) => {
            m.property.name == "length" || m.property.name == "size"
        }
        _ => false,
    }
}

/// Returns `true` when `expr` (after stripping parentheses) is the numeric
/// literal `0`.
fn is_zero(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::NumericLiteral(n) => n.value == 0.0,
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_collection_size_mischeck(&mut self, expr: &BinaryExpression<'_>) {
        // Shape 1: <length_or_size> < 0  (always false)
        // Shape 2: <length_or_size> >= 0 (always true)
        // Shape 3: 0 > <length_or_size>  (always false, mirrors shape 1)
        // Shape 4: 0 <= <length_or_size> (always true,  mirrors shape 2)
        let flagged = matches!(
            expr.operator,
            BinaryOperator::LessThan | BinaryOperator::GreaterEqualThan
        ) && is_length_or_size(&expr.left)
            && is_zero(&expr.right)
            || matches!(
                expr.operator,
                BinaryOperator::GreaterThan | BinaryOperator::LessEqualThan
            ) && is_zero(&expr.left)
                && is_length_or_size(&expr.right);
        if flagged {
            self.report(RULE_NAME, "collectionSizeMischeck", expr.span);
        }
    }
}
