//! Rule `no-nested-assignment` (SonarJS key S1121).
//!
//! Clean-room port. An assignment buried inside a larger expression is easy to
//! misread — `if (x = y)` looks like a comparison `if (x == y)`, and a chained
//! `a = b = c` hides two assignments in one statement. Such assignments should
//! be pulled out into their own statement.
//!
//! ## Narrow form
//!
//! This port reports the two unambiguous nested positions, both detected from
//! the enclosing node so no parent pointers are needed:
//!
//! - an `=` assignment used as the condition of an `if`, `while`, `do…while`, or
//!   `for` statement — `if (x = y) {}`, `while (node = node.next) {}`;
//! - the right-hand side of a chained `=` assignment — the `b = c` in
//!   `a = b = c`.
//!
//! Only the plain `=` operator is reported, which guarantees the flagged code is
//! genuinely a hidden assignment. Other sub-expression positions (call
//! arguments, declarator initializers, array/object literals) and compound
//! assignment operators are a documented follow-up.
//!
//! **Not flagged**: an assignment that is its own expression statement
//! (`x = y;`), or the `init`/`update` clauses of a `for` loop (`for (i = 0; …;
//! i = i + 1)`), where an assignment is the expected form.
//!
//! Behaviour is reproduced from the public RSPEC description (S1121) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{AssignmentExpression, Expression};
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-nested-assignment";

impl Scanner<'_> {
    /// Reports `expr` when it is an `=` assignment whose value is consumed as a
    /// loop or branch condition.
    pub(crate) fn check_no_nested_assignment_condition(&mut self, expr: &Expression<'_>) {
        if let Expression::AssignmentExpression(assign) = expr.get_inner_expression() {
            self.report_nested_assignment(assign);
        }
    }

    /// Reports the right-hand side of a chained `=` assignment (`a = b = c`).
    pub(crate) fn check_no_nested_assignment_chain(&mut self, assign: &AssignmentExpression<'_>) {
        if let Expression::AssignmentExpression(inner) = assign.right.get_inner_expression() {
            self.report_nested_assignment(inner);
        }
    }

    fn report_nested_assignment(&mut self, assign: &AssignmentExpression<'_>) {
        if assign.operator == AssignmentOperator::Assign {
            self.report(RULE_NAME, "nestedAssignment", assign.span);
        }
    }
}
