//! Rule `values-not-convertible-to-numbers` (SonarJS key S3758).
//!
//! "Values not convertible to numbers should not be used in numeric
//! comparisons." A relational comparison (`<`, `>`, `<=`, `>=`) performs a
//! numeric `ToNumber` coercion on its operands. When an operand has no numeric
//! value it coerces to `NaN`, and every relational comparison involving `NaN`
//! evaluates to `false`. The comparison is therefore dead — it can never be
//! `true` — which is almost always a defect (per the public RSPEC, `obj > 24`
//! where `obj = {prop: 42}` is Noncompliant; `obj.prop > 24` is Compliant).
//!
//! ## Operators considered
//!
//! Only the four relational operators that perform numeric coercion: less-than
//! `<`, greater-than `>`, less-than-or-equal `<=`, and greater-than-or-equal
//! `>=`. The equality operators (`==`, `!=`, `===`, `!==`) are EXCLUDED: they do
//! not coerce both operands to numbers (strict equality does no coercion, loose
//! equality follows different rules), so an object on one side does not yield an
//! always-`false` numeric comparison there.
//!
//! ## Operand shapes flagged (zero-false-positive literal subset)
//!
//! Without type/flow inference we can only be CERTAIN an operand always coerces
//! to `NaN` when it is literally a value type that has no numeric representation:
//! an `ObjectExpression` (`{}`/`{a: 1}`), a `FunctionExpression`, or an
//! `ArrowFunctionExpression`. An object literal's default `valueOf` returns the
//! object, and `toString` yields `"[object Object]"` → `NaN`; a function
//! likewise stringifies to its source and coerces to `NaN`. A relational
//! `BinaryExpression` is reported when either operand (after
//! `get_inner_expression`, so parentheses are seen through) is one of these.
//!
//! ## Deliberately excluded
//!
//! - **`ArrayExpression`** — arrays CAN coerce to a meaningful number: `[]`
//!   becomes `0`, `[5]` becomes `5`, so `[] > 0` and `[5] > 0` are well-defined
//!   comparisons, not always-`false` defects. Flagging arrays would be a false
//!   positive.
//! - **String literals** — relational comparison of strings is the separate
//!   rule S3003 (`strings-comparison`); it is a surprising lexicographic
//!   comparison rather than a `NaN` coercion, and is handled there.
//! - **Identifiers / other expressions** — a variable might hold a number, so
//!   flagging the general variable form (e.g. `obj > 24` where `obj` is an
//!   object) requires type/flow inference and would risk false positives. This
//!   is a DELIBERATE under-report: only the unambiguous literal forms are
//!   flagged.
//!
//! ## Flagged
//! - `({}) < 5` — object literal coerces to `NaN`; always false.
//! - `({a: 1}) > 24` — object literal operand.
//! - `(() => {}) > 1` — arrow-function literal operand.
//! - `(function () {}) <= 0` — function-expression operand.
//!
//! ## Not flagged
//! - `[] > 0`, `[5] > 0` — arrays coerce to a defined number.
//! - `"a" < "b"` — string comparison is the separate rule S3003.
//! - `x > 1`, `1 < 2` — identifiers/numbers need type inference or are valid.
//! - `({}) === x` — equality, not a relational numeric comparison.
//!
//! The report covers the span of the whole `BinaryExpression`.
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S3758 description
//! only; no upstream source, tests, fixtures, or message strings were consulted
//! or copied.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "values-not-convertible-to-numbers";

impl<'a> Scanner<'a> {
    /// Reports a relational `BinaryExpression` (`<`, `>`, `<=`, `>=`) where
    /// either operand is an object, function, or arrow-function literal, which
    /// always coerces to `NaN`, making the comparison always `false`.
    pub(crate) fn check_values_not_convertible_to_numbers(&mut self, it: &BinaryExpression<'a>) {
        if !matches!(
            it.operator,
            BinaryOperator::LessThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterEqualThan
        ) {
            return;
        }
        if Self::is_never_numeric_literal(it.left.get_inner_expression())
            || Self::is_never_numeric_literal(it.right.get_inner_expression())
        {
            self.report(RULE_NAME, "valuesNotConvertibleToNumbers", it.span);
        }
    }

    /// Returns `true` when the (parenthesis-unwrapped) operand is literally an
    /// object, function, or arrow-function expression — the only operands we can
    /// be certain coerce to `NaN` without type inference. Arrays are
    /// deliberately excluded (they coerce to a defined number).
    fn is_never_numeric_literal(expr: &Expression<'a>) -> bool {
        matches!(
            expr,
            Expression::ObjectExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::ArrowFunctionExpression(_)
        )
    }
}
