//! Rule `no-incorrect-string-concat` (SonarSource key S3402).
//!
//! Title: "Strings and non-strings should not be added." Using the `+`
//! operator to combine a string with a non-string relies on JavaScript's
//! implicit coercion to string (concatenation). Per the public SonarSource
//! RSPEC S3402, the Noncompliant example `x + z` — where `x` is a number and
//! `z` is a string — silently yields the string `"138"` instead of the
//! intended numeric addition; the Compliant fix converts the operand first
//! with `x + Number(z)`.
//!
//! ## Why the general case needs type inference (deliberate under-report)
//!
//! The RSPEC defect — adding a *number-typed* value to a *string-typed* value
//! — depends on the runtime types of the two operands. In a pure-AST scanner
//! we cannot know whether `x` and `z` are numbers, strings, or something else
//! without type inference, which is unavailable here. Worse, the symmetric
//! shape `string + number` is overwhelmingly *intentional* concatenation in
//! real code (`"count: " + 5`, `"id-" + i`), so flagging it would produce a
//! flood of false positives. The general number/string-variable case is
//! therefore deliberately NOT flagged, keeping this rule zero-false-positive.
//!
//! ## The narrow unambiguous-bug subset
//!
//! What this rule DOES flag is the strict intersection where a coercion can
//! never be intentional: a `+` `BinaryExpression` in which one operand (after
//! `get_inner_expression` unwrapping) is a `StringLiteral` AND the other
//! operand (after unwrapping) is an object, array, function, or arrow-function
//! literal. Concatenating a string with such a complex literal always produces
//! a nonsensical coercion — `"x" + {}` becomes `"x[object Object]"`,
//! `"" + (() => {})` becomes the function's source text — and is never a
//! reasonable thing to write, so it is an unambiguous defect. The pairing is
//! flagged in either order (`literal + string` and `string + literal`).
//!
//! Reported at the span of the entire `BinaryExpression` with message id
//! `incorrectStringConcat`.
//!
//! ## Flagged
//! - `"x" + {}` — string + object literal → `"x[object Object]"`
//! - `[] + "x"` — array literal + string
//! - `"x" + (() => {})` — string + arrow-function literal
//! - `"label" + function () {}` — string + function literal
//!
//! ## Not flagged
//! - `"x" + 5` — string + number: intentional concatenation (needs type inference for the RSPEC defect)
//! - `"a" + "b"` — string + string: ordinary concatenation
//! - `"x" + y` — string + identifier: operand type unknown
//! - `obj + "x"` — non-literal complex operand, type unknown
//! - `"x" - {}` — operator is not `+`
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S3402 only;
//! no upstream source, tests, fixtures, or message strings were consulted.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-incorrect-string-concat";

/// Returns `true` when `expr` (already unwrapped via `get_inner_expression`)
/// is a non-string complex literal — an object, array, function, or
/// arrow-function expression — whose coercion to string in a `+` with a
/// string literal is always a defect.
fn is_non_string_complex_literal(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
    )
}

impl<'a> Scanner<'a> {
    /// Reports a `+` expression that concatenates a string literal with an
    /// object, array, function, or arrow-function literal — an unambiguous
    /// implicit-coercion bug (S3402).
    pub(crate) fn check_no_incorrect_string_concat(&mut self, it: &BinaryExpression<'a>) {
        if it.operator != BinaryOperator::Addition {
            return;
        }
        let left = it.left.get_inner_expression();
        let right = it.right.get_inner_expression();
        let string_with_complex = (matches!(left, Expression::StringLiteral(_))
            && is_non_string_complex_literal(right))
            || (matches!(right, Expression::StringLiteral(_))
                && is_non_string_complex_literal(left));
        if string_with_complex {
            self.report(RULE_NAME, "incorrectStringConcat", it.span);
        }
    }
}
