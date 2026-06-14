//! Rule `bitwise-operators` (SonarJS key S1529).
//!
//! Clean-room port. A bitwise AND (`&`) or bitwise OR (`|`) is very often a
//! typo for the logical `&&` / `||` when one of its operands is a value that is
//! clearly boolean by construction. This rule reports such a `BinaryExpression`.
//!
//! **Trigger (operand model).** The rule fires on a `&` or `|`
//! `BinaryExpression` when at least one operand (after stripping parentheses)
//! is a *boolean-valued* expression:
//!
//! - a comparison `BinaryExpression` (`==`, `!=`, `===`, `!==`, `<`, `<=`,
//!   `>`, `>=`, `instanceof`, `in`) — its result is a boolean;
//! - a logical `LogicalExpression` (`&&`, `||`);
//! - a logical-not `UnaryExpression` (`!`);
//! - a boolean literal (`true` / `false`).
//!
//! This mirrors the SonarJS / SEI CERT EXP46-C model: mixing a bitwise operator
//! with a relational/equality (or otherwise boolean) operand signals that a
//! logical operator was almost certainly intended. The decision is made purely
//! from the operands, so it is independent of the surrounding context (the
//! expression need not be the test of an `if`/`while`/ternary).
//!
//! **Deliberately NOT flagged** to avoid false positives on legitimate
//! bit-manipulation code: bitwise expressions whose operands are plain
//! identifiers, numeric literals, member accesses, or calls (e.g.
//! `x & MASK`, `a | b`, `flags & 0x1`). `^` (XOR) is excluded because it has no
//! logical counterpart in JavaScript and the diagnostic message refers to
//! `&&` / `||`. Compound assignments (`&=`, `|=`) are `AssignmentExpression`
//! nodes and are therefore never considered.
//!
//! The reported span is the whole bitwise `BinaryExpression`.
//!
//! Behaviour is reproduced from the public RSPEC S1529 description and the
//! equivalent SEI CERT EXP46-C guidance only; no upstream source, tests,
//! fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "bitwise-operators";

/// Returns `true` when `expr` (after stripping parentheses) is a boolean-valued
/// expression: a comparison, a logical `&&`/`||`, a logical-not `!`, or a
/// boolean literal. Conservative on purpose — identifier and numeric operands
/// (legitimate bit operations) are never treated as boolean.
fn is_boolean_valued(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::BinaryExpression(bin) => matches!(
            bin.operator,
            BinaryOperator::Equality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictEquality
                | BinaryOperator::StrictInequality
                | BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan
                | BinaryOperator::Instanceof
                | BinaryOperator::In
        ),
        Expression::LogicalExpression(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(unary.operator, UnaryOperator::LogicalNot)
        }
        Expression::BooleanLiteral(_) => true,
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_bitwise_operators(&mut self, expr: &BinaryExpression<'_>) {
        if !matches!(
            expr.operator,
            BinaryOperator::BitwiseAnd | BinaryOperator::BitwiseOR
        ) {
            return;
        }
        if !is_boolean_valued(&expr.left) && !is_boolean_valued(&expr.right) {
            return;
        }
        self.report(RULE_NAME, "bitwiseOperator", expr.span);
    }
}
