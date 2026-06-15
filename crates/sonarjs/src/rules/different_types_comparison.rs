//! Rule `different-types-comparison` (SonarJS key S3403).
//!
//! A strict equality (`===`) or strict inequality (`!==`) between two values
//! whose primitive types are provably different can never vary: `===` is
//! always `false` and `!==` is always `true`. Such a comparison is therefore
//! constant, which is almost always a bug — the developer likely intended to
//! compare values that could actually be equal.
//!
//! ## Conservative zero-false-positive design
//!
//! Only the unambiguous syntactic subset is flagged: a `BinaryExpression`
//! using `===` or `!==` where BOTH operands (after unwrapping parentheses via
//! `get_inner_expression`) are primitive literals whose KINDS differ, among
//! `{string, number, bigint, boolean, null}`. Because the literal kind is
//! known statically, the comparison result is provably constant.
//!
//! Everything else is conservatively not flagged:
//! - Same-kind literal pairs (`1 === 2`, `"a" === "b"`) — the result depends
//!   on the runtime VALUES, not a type mismatch, so this is not a type bug.
//! - Any non-literal operand (identifier, member, call, object, array) — the
//!   type is unknown, so no claim can be made.
//! - Loose `==`/`!=` — coercion can make cross-type comparisons truthy, so the
//!   result is not constant.
//! - `RegExpLiteral`, `ObjectExpression`, `ArrayExpression` — treated as
//!   non-primitive; if either side is one of these the comparison is skipped.
//!
//! Reported at the span of the entire `BinaryExpression`.
//!
//! ## Flagged
//! - `"a" === 1` — string vs number
//! - `null === 0` — null vs number
//! - `true === "x"` — boolean vs string
//! - `5 !== "5"` — number vs string
//! - `1n === 1` — bigint vs number
//!
//! ## Not flagged
//! - `1 === 2` — same kind, value-dependent
//! - `"a" === "b"` — same kind, value-dependent
//! - `x === 1` — left operand is not a literal
//! - `1 == "1"` — loose equality, coercion applies
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S3403 only;
//! no upstream source, tests, fixtures, or message strings were consulted.

use oxc_ast::ast::{BinaryExpression, Expression};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "different-types-comparison";

/// The primitive type a literal operand evaluates to. Two operands with
/// distinct `PrimKind` values can never be strictly equal.
#[derive(PartialEq, Eq)]
enum PrimKind {
    String,
    Number,
    BigInt,
    Boolean,
    Null,
}

/// Maps an expression to its primitive literal kind, or `None` when the
/// expression is not one of the recognised primitive literals (i.e. its type
/// cannot be determined syntactically). Parentheses are assumed already
/// unwrapped by the caller via `get_inner_expression`.
fn prim_kind(expr: &Expression) -> Option<PrimKind> {
    match expr {
        Expression::StringLiteral(_) => Some(PrimKind::String),
        Expression::NumericLiteral(_) => Some(PrimKind::Number),
        Expression::BigIntLiteral(_) => Some(PrimKind::BigInt),
        Expression::BooleanLiteral(_) => Some(PrimKind::Boolean),
        Expression::NullLiteral(_) => Some(PrimKind::Null),
        _ => None,
    }
}

impl<'a> Scanner<'a> {
    /// Reports a strict `===`/`!==` comparison whose two operands are primitive
    /// literals of provably different kinds, making the comparison constant.
    pub(crate) fn check_different_types_comparison(&mut self, it: &BinaryExpression<'a>) {
        if !matches!(
            it.operator,
            BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
        ) {
            return;
        }
        let left = prim_kind(it.left.get_inner_expression());
        let right = prim_kind(it.right.get_inner_expression());
        match (left, right) {
            (Some(left), Some(right)) if left != right => {
                self.report(RULE_NAME, "differentTypesComparison", it.span);
            }
            _ => {}
        }
    }
}
