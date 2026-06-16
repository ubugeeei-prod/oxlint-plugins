//! Rule `argument-type` (SonarJS key S3782).
//!
//! Clean-room port. Built-in functions document the types of arguments they
//! accept; passing a value of the wrong type is almost always a bug. The
//! canonical RSPEC S3782 example is `Math.abs(x < 0.0042)`: the comparison
//! `x < 0.0042` produces a boolean, but `Math.abs` expects a number, so the
//! parentheses are misplaced (the author meant `Math.abs(x) < 0.0042`).
//!
//! This implements ONLY the unambiguous, zero-false-positive subset of the
//! rule: a single-argument numeric `Math.*` method called with an argument
//! whose value is *unambiguously a boolean*. The Math methods covered are the
//! single-numeric-argument ones —
//! `abs, acos, acosh, asin, asinh, atan, atanh, cbrt, ceil, clz32, cos, cosh,
//! exp, expm1, floor, fround, log, log10, log1p, log2, round, sign, sin, sinh,
//! sqrt, tan, tanh, trunc`. Methods that take more than one argument or are
//! variadic (`atan2`, `pow`, `hypot`, `max`, `min`, `imul`) and the
//! zero-argument `random` are deliberately excluded, because their argument
//! shapes are different and a boolean there is harder to call a clear mistake.
//!
//! An argument counts as boolean-producing when, after stripping parentheses,
//! it is one of:
//! - a `BinaryExpression` with a comparison operator
//!   (`<`, `>`, `<=`, `>=`, `==`, `===`, `!=`, `!==`);
//! - a `LogicalExpression` (`&&` or `||`); or
//! - a `UnaryExpression` with the logical-not operator (`!`).
//!
//! Numbers, identifiers, and any other expression are never flagged: a number
//! is the correct type, and an identifier's type is unknown without type
//! analysis. Requiring a boolean-producing argument keeps this
//! zero-false-positive.
//!
//! ## Flagged
//! ```js
//! Math.abs(x < 0.0042);   // comparison -> boolean
//! Math.floor(a && b);     // logical    -> boolean
//! Math.sqrt(!ready);      // logical-not -> boolean
//! ```
//!
//! ## Not flagged
//! ```js
//! Math.abs(x);            // identifier (unknown type)
//! Math.abs(x) < 0.0042;   // comparison is outside the call
//! Math.floor(1.5);        // numeric argument (correct type)
//! Math.max(a < b, c);     // max is excluded (multi-argument)
//! Math.atan2(a, b);       // atan2 is excluded (two arguments)
//! foo.abs(x < 1);         // object is not the `Math` identifier
//! ```
//!
//! Behaviour is reproduced from the public RSPEC S3782 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "argument-type";

/// Returns `true` for the single-numeric-argument `Math.*` method names.
/// Variadic / multi-argument members (`atan2`, `pow`, `hypot`, `max`, `min`,
/// `imul`) and `random` are intentionally omitted.
fn is_single_number_math_method(name: &str) -> bool {
    matches!(
        name,
        "abs"
            | "acos"
            | "acosh"
            | "asin"
            | "asinh"
            | "atan"
            | "atanh"
            | "cbrt"
            | "ceil"
            | "clz32"
            | "cos"
            | "cosh"
            | "exp"
            | "expm1"
            | "floor"
            | "fround"
            | "log"
            | "log10"
            | "log1p"
            | "log2"
            | "round"
            | "sign"
            | "sin"
            | "sinh"
            | "sqrt"
            | "tan"
            | "tanh"
            | "trunc"
    )
}

/// Returns `true` when `expr` (already unwrapped of parentheses) is an
/// expression that unambiguously evaluates to a boolean.
fn is_boolean_producing(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::BinaryExpression(bin) => matches!(
            bin.operator,
            BinaryOperator::LessThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterEqualThan
                | BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
        ),
        Expression::LogicalExpression(_) => true,
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::LogicalNot,
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_argument_type(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        if object.name != "Math" || !is_single_number_math_method(member.property.name.as_str()) {
            return;
        }
        if it.arguments.len() != 1 {
            return;
        }
        let Some(arg) = it.arguments[0].as_expression() else {
            return;
        };
        if is_boolean_producing(arg.get_inner_expression()) {
            self.report(RULE_NAME, "argumentType", it.span);
        }
    }
}
