//! Rule `strings-comparison` (SonarSource key S3003).
//!
//! Applying a relational comparison operator — `<`, `>`, `<=`, `>=` — to
//! strings performs a lexicographic (character-by-character) comparison, not
//! a numeric one. This is a common source of logic errors: for example
//! `"123" < "45"` evaluates to `true` because `'1'` sorts before `'4'`, even
//! though `123` is numerically greater than `45`. When numeric ordering is
//! intended the operands should be converted with `Number(...)` first.
//!
//! ## Conservative zero-false-positive design
//!
//! Without type inference we cannot know the runtime type of a variable or
//! expression, so the only case where we can be CERTAIN both operands are
//! strings is when both are literally string literals. This rule therefore
//! flags a `BinaryExpression` whose operator is one of the relational
//! comparisons (`<`, `>`, `<=`, `>=`) and whose BOTH operands (after
//! unwrapping parentheses via `get_inner_expression`) are
//! `Expression::StringLiteral`.
//!
//! The far more common real-world form — comparing two string-typed
//! variables (`appleNumber < orangeNumber`, the verbatim RSPEC Noncompliant
//! example) — is deliberately UNDER-REPORTED here, because proving that the
//! operands are strings requires type inference that is unavailable in this
//! pure-AST scanner. Flagging the variable form without type information would
//! produce false positives on numeric or mixed comparisons, so it is omitted.
//! The literal subset is the strict zero-false-positive intersection.
//!
//! Equality operators are never flagged: `==`, `===`, `!=`, `!==` behave
//! correctly on strings and comparing strings for equality is a legitimate,
//! common operation. The `+` operator (string concatenation) is likewise not a
//! relational comparison and is not flagged.
//!
//! Reported at the span of the entire `BinaryExpression`.
//!
//! ## Flagged
//! - `"123" < "45"` — lexicographic comparison surprises (`true`)
//! - `"a" >= "b"` — both operands are string literals
//! - `"x" > "y"`, `"a" <= "b"`
//!
//! ## Not flagged
//! - `"a" === "b"` / `"a" == "b"` / `"a" !== "b"` — equality is fine on strings
//! - `1 < 2` — numeric literals, correct numeric comparison
//! - `"a" < x` — right operand is not a string literal (type unknown)
//! - `appleNumber < orangeNumber` — variable form, needs type inference
//! - `"a" + "b"` — concatenation, not a relational comparison
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S3003 only;
//! no upstream source, tests, fixtures, or message strings were consulted.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "strings-comparison";

impl<'a> Scanner<'a> {
    /// Reports a relational comparison (`<`, `>`, `<=`, `>=`) where both
    /// operands are string literals, which performs a surprising lexicographic
    /// comparison rather than a numeric one.
    pub(crate) fn check_strings_comparison(&mut self, it: &BinaryExpression<'a>) {
        if !matches!(
            it.operator,
            BinaryOperator::LessThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterEqualThan
        ) {
            return;
        }
        let left_is_string = matches!(it.left.get_inner_expression(), Expression::StringLiteral(_));
        let right_is_string = matches!(
            it.right.get_inner_expression(),
            Expression::StringLiteral(_)
        );
        if left_is_string && right_is_string {
            self.report(RULE_NAME, "stringsComparison", it.span);
        }
    }
}
