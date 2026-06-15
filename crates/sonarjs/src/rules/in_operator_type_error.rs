//! Rule `in-operator-type-error` (SonarJS key S3785).
//!
//! The `in` operator requires its right-hand operand to be an object: at
//! runtime the engine looks up a property KEY on that object. When the right
//! operand evaluates to a primitive value the expression throws a
//! `TypeError` ("Cannot use 'in' operator to search for ... in ...") every
//! time it executes, so the code is certainly broken.
//!
//! ## Conservative zero-false-positive design
//!
//! Only flagged when the right-hand operand (after unwrapping any enclosing
//! parentheses via `get_inner_expression`) is a PRIMITIVE LITERAL whose value
//! can never be an object: a `StringLiteral`, `NumericLiteral`,
//! `BigIntLiteral`, `BooleanLiteral`, or `NullLiteral`. Each of these always
//! throws a `TypeError` at runtime regardless of the left operand.
//!
//! Everything else is conservatively not flagged to avoid false positives:
//! - `ObjectExpression` / `ArrayExpression` / `RegExpLiteral` are objects, so
//!   the operator is legal.
//! - Any identifier, member access, call, or template literal has an unknown
//!   value at lint time. In particular `undefined` is a shadowable identifier
//!   rather than a literal, so `x in undefined` is intentionally skipped.
//!
//! Reported at the span of the entire `BinaryExpression`.
//!
//! ## Flagged
//! - `"a" in "s"` — string right operand
//! - `0 in 5` — numeric right operand
//! - `k in null` — null right operand
//! - `x in true` — boolean right operand
//!
//! ## Not flagged
//! - `"x" in obj` — right operand is an identifier (value unknown)
//! - `"x" in {}` — right operand is an object literal (legal)
//! - `"x" in []` — right operand is an array literal (legal)
//! - `key in foo.bar` — right operand is a member access (value unknown)
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S3785 only;
//! no upstream source, tests, fixtures, or message strings were consulted.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "in-operator-type-error";

impl<'a> Scanner<'a> {
    /// Reports a `BinaryExpression` using the `in` operator whose right-hand
    /// operand is a primitive literal, which always throws a `TypeError` at
    /// runtime.
    pub(crate) fn check_in_operator_type_error(&mut self, it: &BinaryExpression<'a>) {
        if it.operator != BinaryOperator::In {
            return;
        }
        if is_primitive_literal(it.right.get_inner_expression()) {
            self.report(RULE_NAME, "inOperatorTypeError", it.span);
        }
    }
}

/// Returns `true` when `expr` is a primitive literal that can never be an
/// object — using it as the right-hand operand of `in` always throws.
fn is_primitive_literal(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
}
