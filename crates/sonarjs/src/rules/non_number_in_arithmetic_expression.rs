//! Rule `non-number-in-arithmetic-expression` (SonarJS key S3760).
//!
//! "Arithmetic operators should only have numbers as operands." Applying an
//! arithmetic operator to a non-numeric operand (a string or a boolean) relies
//! on JavaScript's implicit type conversion, which is error-prone and usually
//! signals a mistake (`"80" / 4`, `true * 2`). This port implements the
//! conservative, purely syntactic subset of S3760 that can be flagged with zero
//! false positives.
//!
//! ## Operators considered
//!
//! The arithmetic operators that perform a numeric `ToNumber` coercion on both
//! operands: subtraction `-`, multiplication `*`, division `/`, remainder `%`,
//! and exponentiation `**`. The addition operator `+` is deliberately EXCLUDED:
//! per the RSPEC, binary `+` with a string operand is intentional string
//! concatenation, not arithmetic, so it is never flagged here. Comparison and
//! bitwise operators are also out of scope (string-to-string comparison is the
//! separate rule S3003).
//!
//! ## Operand shapes flagged (zero-false-positive literal subset)
//!
//! Without type inference we can only be CERTAIN an operand is non-numeric when
//! it is literally a string or a boolean. A `BinaryExpression` is therefore
//! reported when its operator is arithmetic-but-not-`+` AND either operand
//! (after `get_inner_expression`, so parentheses are seen through) is a
//! `StringLiteral` or a `BooleanLiteral`.
//!
//! The typed-variable form — e.g. a `string`/`boolean`-typed identifier used as
//! an operand — requires type inference to detect and is a DELIBERATE
//! under-report: flagging an arbitrary identifier would produce false positives
//! (the variable may well hold a number), so only the literal form is flagged.
//!
//! ## Flagged
//! - `"80" / 4` — string literal under division.
//! - `true * 2` — boolean literal under multiplication.
//! - `5 - "1"` — string literal under subtraction.
//! - `2 ** false` — boolean literal under exponentiation.
//!
//! ## Not flagged
//! - `"a" + "b"`, `"x" + 1` — the `+` operator is excluded (concatenation).
//! - `5 / 4`, `2 * 3` — both operands numeric.
//! - `x / 4`, `a * b` — identifiers; the variable form needs type inference.
//! - `a < "b"`, `x & 1` — comparison and bitwise operators are out of scope.
//!
//! The report covers the span of the whole `BinaryExpression`.
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S3760 description
//! only; no upstream source, tests, fixtures, or message strings were consulted
//! or copied.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "non-number-in-arithmetic-expression";

impl<'a> Scanner<'a> {
    /// Reports an arithmetic `BinaryExpression` (`-`, `*`, `/`, `%`, `**`, but
    /// NOT `+`) where either operand is literally a string or boolean, which
    /// relies on implicit numeric conversion.
    pub(crate) fn check_non_number_in_arithmetic_expression(&mut self, it: &BinaryExpression<'a>) {
        if !matches!(
            it.operator,
            BinaryOperator::Subtraction
                | BinaryOperator::Multiplication
                | BinaryOperator::Division
                | BinaryOperator::Remainder
                | BinaryOperator::Exponential
        ) {
            return;
        }
        if Self::non_number_in_arithmetic_operand_is_non_numeric_literal(
            it.left.get_inner_expression(),
        ) || Self::non_number_in_arithmetic_operand_is_non_numeric_literal(
            it.right.get_inner_expression(),
        ) {
            self.report(RULE_NAME, "nonNumberInArithmetic", it.span);
        }
    }

    /// Returns `true` when the (parenthesis-unwrapped) operand is literally a
    /// string or boolean — the only operands we can be certain are non-numeric
    /// without type inference.
    fn non_number_in_arithmetic_operand_is_non_numeric_literal(expr: &Expression<'a>) -> bool {
        matches!(
            expr,
            Expression::StringLiteral(_) | Expression::BooleanLiteral(_)
        )
    }
}
