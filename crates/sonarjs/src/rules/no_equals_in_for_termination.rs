//! Rule `no-equals-in-for-termination` (SonarJS key S888).
//!
//! Clean-room port. Using an equality operator (`==`, `!=`, `===`, `!==`) in a
//! `for` loop's termination condition is risky: if the loop counter steps *past*
//! the bound it never compares equal to it and the loop runs forever:
//!
//! ```js
//! for (let i = 1; i != 10; i += 2) {}   // Noncompliant: i goes 1,3,5,7,9,11,... never 10
//! ```
//!
//! The well-known S888 exception (originating in CERT MSC21) is that a counter
//! moved by a *unit* step (`++`/`--`, or `+= 1`/`-= 1`) can never skip its target
//! value, so an equality test on such a counter is safe and is NOT flagged:
//!
//! ```js
//! for (let i = 0; i != 10; i++) {}      // Compliant: unit step cannot overshoot
//! ```
//!
//! ## Conservative scope (fewest false positives)
//!
//! The rule fires only when it can positively identify a risky, non-unit step on
//! a counter that participates in the equality test. Concretely it reports iff:
//!
//! - The `test` is a `BinaryExpression` whose operator is one of `==`, `!=`,
//!   `===`, `!==`, and at least one operand is a plain `Identifier` (a candidate
//!   counter).
//! - The `update` clause moves that same counter identifier by a *recognised
//!   additive step* whose magnitude is not 1:
//!   - `i += k` / `i -= k` with `k` a numeric literal other than `1`, or a
//!     non-literal amount.
//!   - `i = i + k` / `i = k + i` / `i = i - k` with `k` a numeric literal other
//!     than `1`.
//!
//! Everything else is left unflagged: unit steps (`++`/`--`/`+= 1`/`-= 1`), a
//! relational test (no equality), a counter that is not updated in the header,
//! an update on a different variable, an opaque update (`i = i.next`, a call),
//! a missing test, or a missing update. This deliberately favours avoiding
//! false positives on safe loops (e.g. counters advanced in the loop body) over
//! catching exotic risky shapes.
//!
//! Behaviour is reproduced from the public RSPEC description (S888) and the
//! observable behaviour of the equivalent check only; no upstream source, tests,
//! fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{AssignmentTarget, Expression, ForStatement, SimpleAssignmentTarget};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-equals-in-for-termination";

/// Classification of the loop counter's step magnitude.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    /// A `++`/`--`/`±= 1`/`= i ± 1` step that cannot overshoot the bound.
    Unit,
    /// A recognised additive step whose magnitude is provably not 1.
    NonUnit,
}

/// Returns `true` when `test` is a `BinaryExpression` with an equality operator
/// whose left or right operand (after stripping parentheses) names `name`.
fn equality_mentions(test: &Expression<'_>, name: &str) -> bool {
    let Expression::BinaryExpression(bin) = test.get_inner_expression() else {
        return false;
    };
    let is_equality = matches!(
        bin.operator,
        BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
    );
    if !is_equality {
        return false;
    }
    is_named_identifier(&bin.left, name) || is_named_identifier(&bin.right, name)
}

/// Returns `true` when `expr` (after stripping parentheses) is an identifier
/// named `name`.
fn is_named_identifier(expr: &Expression<'_>, name: &str) -> bool {
    matches!(
        expr.get_inner_expression(),
        Expression::Identifier(id) if id.name.as_str() == name
    )
}

/// Returns `true` when `expr` is the numeric literal `1`.
fn is_literal_one(expr: &Expression<'_>) -> bool {
    matches!(expr.get_inner_expression(), Expression::NumericLiteral(lit) if lit.value == 1.0)
}

/// Classifies the step of an `i = <expr>` assignment, where `name` is the
/// assigned identifier. Recognises `i + k`, `k + i`, and `i - k`; anything else
/// is an opaque update and yields `None`.
fn assign_rhs_step(rhs: &Expression<'_>, name: &str) -> Option<Step> {
    let Expression::BinaryExpression(bin) = rhs.get_inner_expression() else {
        return None;
    };
    match bin.operator {
        BinaryOperator::Addition => {
            if is_named_identifier(&bin.left, name) {
                return Some(unit_or_not(is_literal_one(&bin.right)));
            }
            if is_named_identifier(&bin.right, name) {
                return Some(unit_or_not(is_literal_one(&bin.left)));
            }
            None
        }
        BinaryOperator::Subtraction => {
            if is_named_identifier(&bin.left, name) {
                return Some(unit_or_not(is_literal_one(&bin.right)));
            }
            None
        }
        _ => None,
    }
}

fn unit_or_not(is_one: bool) -> Step {
    if is_one { Step::Unit } else { Step::NonUnit }
}

/// Determines the variable moved by the `update` clause and the magnitude of its
/// step, for the recognised additive shapes. Returns `None` for any update whose
/// stepped variable or magnitude cannot be determined (calls, member
/// assignments, `*=`, etc.).
fn update_step<'a>(update: &'a Expression<'a>) -> Option<(&'a str, Step)> {
    match update.get_inner_expression() {
        Expression::UpdateExpression(u) => {
            let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &u.argument else {
                return None;
            };
            // `++`/`--` are always unit steps regardless of direction.
            Some((target.name.as_str(), Step::Unit))
        }
        Expression::AssignmentExpression(a) => {
            let AssignmentTarget::AssignmentTargetIdentifier(target) = &a.left else {
                return None;
            };
            let name = target.name.as_str();
            match a.operator {
                AssignmentOperator::Addition | AssignmentOperator::Subtraction => {
                    Some((name, unit_or_not(is_literal_one(&a.right))))
                }
                AssignmentOperator::Assign => assign_rhs_step(&a.right, name).map(|s| (name, s)),
                _ => None,
            }
        }
        _ => None,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_equals_in_for_termination(&mut self, stmt: &ForStatement<'_>) {
        let Some(test) = &stmt.test else {
            return;
        };
        let Some(update) = &stmt.update else {
            return;
        };
        let Some((counter, step)) = update_step(update) else {
            return;
        };
        if step != Step::NonUnit {
            return;
        }
        if !equality_mentions(test, counter) {
            return;
        }
        self.report(RULE_NAME, "noEqualsInForTermination", stmt.span);
    }
}
