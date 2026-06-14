//! Rule `no-useless-increment` (SonarJS key S2123).
//!
//! Clean-room port. Assigning a *postfix* increment or decrement of a variable
//! back to that same variable wastes the operation: `i = i++` evaluates the old
//! value of `i`, increments `i`, and then writes the old value back — so `i` is
//! left unchanged and the `++` accomplishes nothing.
//!
//! ```js
//! i = i++;   // Noncompliant: i is unchanged
//! j = j--;   // Noncompliant
//! ```
//!
//! **Not flagged**:
//! - `i = ++i` — the *prefix* form returns the incremented value, so the
//!   assignment does change `i` (though it is still redundant with `++i`).
//! - `i = j++` — different variables; the assignment is meaningful.
//! - `i++;` — a standalone update statement.
//!
//! Narrow form: only a plain identifier assigned its own postfix update is
//! reported. The member-expression case (`a.b = a.b++`) requires full operand
//! equivalence and is a documented follow-up; restricting to identifiers
//! guarantees no false positives (e.g. `a.b = c.b++` is not flagged).
//!
//! Behaviour is reproduced from the public RSPEC description (S2123) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{AssignmentExpression, AssignmentTarget, Expression, SimpleAssignmentTarget};
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-useless-increment";

impl Scanner<'_> {
    pub(crate) fn check_no_useless_increment(&mut self, assign: &AssignmentExpression<'_>) {
        if assign.operator != AssignmentOperator::Assign {
            return;
        }
        let AssignmentTarget::AssignmentTargetIdentifier(left) = &assign.left else {
            return;
        };
        let Expression::UpdateExpression(update) = assign.right.get_inner_expression() else {
            return;
        };
        if update.prefix {
            return;
        }
        let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &update.argument else {
            return;
        };
        if left.name == target.name {
            self.report(RULE_NAME, "uselessIncrement", assign.span);
        }
    }
}
