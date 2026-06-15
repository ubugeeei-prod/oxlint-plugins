//! Rule `operation-returning-nan` (SonarJS key S3757).
//!
//! An arithmetic operation applied to an operand that JavaScript cannot convert
//! to a meaningful number silently produces `NaN` rather than raising an error.
//! Because `NaN` then poisons every downstream computation, such an operation is
//! almost always a bug. This port implements the conservative, purely syntactic
//! subset of S3757 that can be flagged with zero false positives.
//!
//! ## Operators considered
//!
//! Only the arithmetic operators that perform a numeric `ToNumber` coercion on
//! BOTH operands are considered: subtraction `-`, multiplication `*`, division
//! `/`, remainder `%`, and exponentiation `**`. The addition operator `+` is
//! deliberately EXCLUDED: `+` is overloaded for string concatenation and invokes
//! `ToPrimitive` (preferring string), so `"a" + {}` yields `"a[object Object]"`
//! and `[] + 1` yields `"1"` — neither is reliably `NaN`. Including `+` would
//! produce false positives, so it is never flagged here.
//!
//! ## Operand shapes flagged
//!
//! A `BinaryExpression` is reported when its operator is arithmetic-but-not-`+`
//! and EITHER operand (after `get_inner_expression`, so parentheses are seen
//! through) is one of:
//!
//! - a `FunctionExpression`, `ArrowFunctionExpression`, or `ClassExpression` —
//!   `ToNumber(function)` / `ToNumber(class)` is ALWAYS `NaN`; or
//! - a PLAIN object literal — an `ObjectExpression` that declares none of the
//!   primitive-conversion hooks `valueOf`, `toString`, or a computed key (which
//!   could be `[Symbol.toPrimitive]`). A custom hook can make the object coerce
//!   to a finite number (`{ valueOf() { return 5; } } * 2 === 10`), so an object
//!   literal carrying any such property — or any spread element, whose contents
//!   are unknown — is treated as NOT plain and conservatively skipped. The
//!   guard intentionally under-reports rather than risk a false positive.
//!
//! ## Flagged
//! - `(() => {}) * 2` — arrow function coerces to `NaN`.
//! - `(function () {}) - 1` — function expression coerces to `NaN`.
//! - `({}) * 2` — empty object literal coerces to `NaN`.
//! - `({ a: 1 }) / 2` — plain data object with no conversion hook.
//!
//! ## Not flagged
//! - `({ valueOf() { return 5; } }) * 2` — custom `valueOf` yields a finite
//!   number, so the object is not plain.
//! - `({ toString() { return "5"; } }) * 2` — custom `toString` likewise.
//! - `[] * 2`, `[5] * 2` — array literals are not reliably `NaN` (`[] * 2 === 0`,
//!   `[5] * 2 === 10`), so `ArrayExpression` is skipped.
//! - `x * 2` — an identifier (e.g. a shadowable `undefined`) is never assumed.
//! - `"a" + {}`, `1 + 2` — the `+` operator is excluded entirely.
//! - string/number/template literals on either side.
//!
//! The report covers the span of the whole `BinaryExpression`.
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S3757 description
//! only; no upstream source, tests, fixtures, or message strings were consulted
//! or copied.

use oxc_ast::ast::{
    BinaryExpression, Expression, ObjectExpression, ObjectPropertyKind, PropertyKey,
};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "operation-returning-nan";

/// Returns `true` when `name` is a primitive-conversion hook that could make an
/// object literal coerce to a finite number, disqualifying it from the
/// "plain object" classification.
fn is_conversion_hook(name: &str) -> bool {
    name == "valueOf" || name == "toString"
}

impl<'a> Scanner<'a> {
    /// Reports an arithmetic `BinaryExpression` (excluding `+`) where either
    /// operand is a function/class expression or a plain object literal, both of
    /// which always coerce to `NaN` under numeric arithmetic.
    pub(crate) fn check_operation_returning_nan(&mut self, it: &BinaryExpression<'a>) {
        if !it.operator.is_arithmetic() || it.operator == BinaryOperator::Addition {
            return;
        }
        let left = it.left.get_inner_expression();
        let right = it.right.get_inner_expression();
        if Self::operation_returning_nan_operand_is_nan(left)
            || Self::operation_returning_nan_operand_is_nan(right)
        {
            self.report(RULE_NAME, "operationReturningNan", it.span);
        }
    }

    /// Decides whether a single (parenthesis-unwrapped) operand always coerces
    /// to `NaN` under numeric arithmetic.
    fn operation_returning_nan_operand_is_nan(expr: &Expression<'a>) -> bool {
        match expr {
            Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_) => true,
            Expression::ObjectExpression(obj) => Self::operation_returning_nan_is_plain_object(obj),
            _ => false,
        }
    }

    /// Returns `true` only when `obj` is a plain object literal: it declares no
    /// `valueOf` / `toString` property, no computed key (which could be a
    /// `[Symbol.toPrimitive]` hook), and no spread element. Any uncertainty is
    /// resolved as "not plain", so the caller under-reports.
    fn operation_returning_nan_is_plain_object(obj: &ObjectExpression<'a>) -> bool {
        for property in &obj.properties {
            let prop = match property {
                ObjectPropertyKind::ObjectProperty(prop) => prop,
                // Spread contents are unknown; treat the object as not plain.
                ObjectPropertyKind::SpreadProperty(_) => return false,
            };
            // A computed key could be `[Symbol.toPrimitive]`; be conservative.
            if prop.computed {
                return false;
            }
            match &prop.key {
                PropertyKey::StaticIdentifier(id) if is_conversion_hook(id.name.as_str()) => {
                    return false;
                }
                PropertyKey::StringLiteral(lit) if is_conversion_hook(lit.value.as_str()) => {
                    return false;
                }
                _ => {}
            }
        }
        true
    }
}
