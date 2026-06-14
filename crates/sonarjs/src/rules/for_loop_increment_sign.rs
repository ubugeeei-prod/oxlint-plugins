//! Rule `for-loop-increment-sign` (SonarJS key S2251).
//!
//! Clean-room port. A `for` loop's update clause should move the loop counter
//! in the direction that approaches its termination condition. When the update
//! pushes the counter *away* from the bound implied by the test, the loop runs
//! unexpectedly (often forever):
//!
//! ```js
//! for (let i = 0; i < 10; i--) {}   // Noncompliant: test wants i to grow, i-- shrinks it
//! for (let i = 10; i > 0; i++) {}   // Noncompliant: test wants i to shrink, i++ grows it
//! ```
//!
//! ## Conservative scope
//!
//! The check fires only when both the test and the update have a clearly
//! recognised, unambiguous shape:
//!
//! - The test is a `BinaryExpression` with a relational operator and the
//!   counter operand is a plain identifier:
//!   - `i < n` / `i <= n` (or mirrored `n > i` / `n >= i`) ⇒ should INCREASE.
//!   - `i > n` / `i >= n` (or mirrored `n < i` / `n <= i`) ⇒ should DECREASE.
//!   - Equality operators (`==`, `!=`, `===`, `!==`) imply no direction and are
//!     never flagged.
//! - The update is one of:
//!   - `i++` / `++i` ⇒ INCREASE; `i--` / `--i` ⇒ DECREASE.
//!   - `i += <expr>` ⇒ INCREASE; `i -= <expr>` ⇒ DECREASE (the operator sign
//!     gives the direction; the right-hand value is not evaluated, so the rare
//!     `i += -1` is intentionally treated as an increase to avoid guessing).
//! - The update variable must be the SAME identifier that appears as the
//!   counter in the test.
//!
//! Anything outside these shapes (non-constant or compound updates, a different
//! update variable, equality conditions, missing test/update) is left
//! unflagged.
//!
//! Behaviour is reproduced from the public RSPEC description (S2251) and the
//! observable behaviour of the equivalent direction check only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{AssignmentTarget, Expression, ForStatement, SimpleAssignmentTarget};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UpdateOperator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "for-loop-increment-sign";

/// The direction in which a counter moves, or in which a condition requires it
/// to move to terminate.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Increase,
    Decrease,
}

impl Direction {
    fn flip(self) -> Self {
        match self {
            Direction::Increase => Direction::Decrease,
            Direction::Decrease => Direction::Increase,
        }
    }
}

/// Returns `true` when `expr` (after stripping parentheses) is an identifier
/// named `name`.
fn is_named_identifier(expr: &Expression<'_>, name: &str) -> bool {
    matches!(
        expr.get_inner_expression(),
        Expression::Identifier(id) if id.name.as_str() == name
    )
}

/// Direction the update moves the counter, paired with the counter name, for
/// the recognised simple update shapes. `None` for anything else.
fn update_direction<'a>(update: &'a Expression<'a>) -> Option<(&'a str, Direction)> {
    match update.get_inner_expression() {
        Expression::UpdateExpression(u) => {
            let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &u.argument else {
                return None;
            };
            let dir = match u.operator {
                UpdateOperator::Increment => Direction::Increase,
                UpdateOperator::Decrement => Direction::Decrease,
            };
            Some((target.name.as_str(), dir))
        }
        Expression::AssignmentExpression(a) => {
            let AssignmentTarget::AssignmentTargetIdentifier(target) = &a.left else {
                return None;
            };
            let dir = match a.operator {
                AssignmentOperator::Addition => Direction::Increase,
                AssignmentOperator::Subtraction => Direction::Decrease,
                _ => return None,
            };
            Some((target.name.as_str(), dir))
        }
        _ => None,
    }
}

/// Direction the test requires `counter` to move to terminate, or `None` when
/// the test is not a relational comparison whose counter operand is the named
/// identifier.
fn condition_direction(test: &Expression<'_>, counter: &str) -> Option<Direction> {
    let Expression::BinaryExpression(bin) = test.get_inner_expression() else {
        return None;
    };
    let left_dir = match bin.operator {
        BinaryOperator::LessThan | BinaryOperator::LessEqualThan => Direction::Increase,
        BinaryOperator::GreaterThan | BinaryOperator::GreaterEqualThan => Direction::Decrease,
        _ => return None,
    };
    if is_named_identifier(&bin.left, counter) {
        return Some(left_dir);
    }
    if is_named_identifier(&bin.right, counter) {
        return Some(left_dir.flip());
    }
    None
}

impl Scanner<'_> {
    pub(crate) fn check_for_loop_increment_sign(&mut self, stmt: &ForStatement<'_>) {
        let Some(test) = &stmt.test else {
            return;
        };
        let Some(update) = &stmt.update else {
            return;
        };
        let Some((counter, update_dir)) = update_direction(update) else {
            return;
        };
        let Some(cond_dir) = condition_direction(test, counter) else {
            return;
        };
        if cond_dir != update_dir {
            self.report(RULE_NAME, "wrongDirection", stmt.span);
        }
    }
}
