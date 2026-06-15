//! Rule `no-identical-expressions` (SonarJS key S1764).
//!
//! Clean-room port. Reports a binary or logical expression where the left and
//! right operands are textually identical AND the operator belongs to a targeted
//! set where having the same sub-expression on both sides is almost certainly a
//! bug (the result would be constant or redundant).
//!
//! ## Targeted operators
//!
//! ### Binary (`BinaryExpression`)
//! - Comparisons: `<` (`LessThan`), `<=` (`LessEqualThan`), `>` (`GreaterThan`),
//!   `>=` (`GreaterEqualThan`), `==` (`Equality`), `===` (`StrictEquality`),
//!   `!=` (`Inequality`), `!==` (`StrictInequality`)
//! - Bitwise: `&` (`BitwiseAnd`), `|` (`BitwiseOR`), `^` (`BitwiseXOR`)
//! - Arithmetic: `-` (`Subtraction`), `/` (`Division`), `%` (`Remainder`)
//!
//! ### Logical (`LogicalExpression`)
//! - `&&` (`LogicalOperator::And`), `||` (`LogicalOperator::Or`)
//!
//! ## Explicitly excluded operators (not a bug with identical operands)
//! - `+` — `a + a` is legitimate doubling.
//! - `*` — `a * a` is legitimate squaring.
//! - `**` — exponentiation; `a ** a` is intentional.
//! - `<<`, `>>`, `>>>` — bit-shifts with identical operands are unusual but
//!   not clearly bugs.
//! - `??` — nullish-coalescing with identical operands may be intentional.
//! - `instanceof`, `in` — not meaningful to flag with identical operands.
//!
//! ## Operand identity
//! Two operands are "identical" iff `self.text(left.span()) == self.text(right.span())`.
//! This is a **syntactic** (source-text) comparison; it does not account for
//! semantic equivalence or aliasing.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{BinaryExpression, LogicalExpression};
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-identical-expressions";

impl Scanner<'_> {
    pub(crate) fn check_no_identical_expressions_binary(&mut self, expr: &BinaryExpression<'_>) {
        let is_targeted = matches!(
            expr.operator,
            BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan
                | BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
                | BinaryOperator::BitwiseAnd
                | BinaryOperator::BitwiseOR
                | BinaryOperator::BitwiseXOR
                | BinaryOperator::Subtraction
                | BinaryOperator::Division
                | BinaryOperator::Remainder
        );
        if !is_targeted {
            return;
        }
        let left = self.text(expr.left.span());
        let right = self.text(expr.right.span());
        if left == right {
            self.report(RULE_NAME, "identicalExpressions", expr.span);
        }
    }

    pub(crate) fn check_no_identical_expressions_logical(&mut self, expr: &LogicalExpression<'_>) {
        if !matches!(expr.operator, LogicalOperator::And | LogicalOperator::Or) {
            return;
        }
        let left = self.text(expr.left.span());
        let right = self.text(expr.right.span());
        if left == right {
            self.report(RULE_NAME, "identicalExpressions", expr.span);
        }
    }
}
