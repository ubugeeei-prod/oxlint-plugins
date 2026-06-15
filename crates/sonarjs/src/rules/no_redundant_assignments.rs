//! Rule `no-redundant-assignments` (SonarJS key S4165).
//!
//! Clean-room port. An assignment is redundant when its value is never used
//! before being overwritten.  Two flavours are detected here:
//!
//! 1. **Self-assignment** — `x = x;` where both sides of a plain `=` are the
//!    exact same identifier.
//! 2. **Adjacent dead reassignment** — two consecutive `ExpressionStatement`s
//!    in the same statement list, both plain `=` assignments to the same plain
//!    identifier LHS (`x = a; x = b;`), with no statement in between, and the
//!    second assignment's RHS does not reference `x`.
//!
//! ```js
//! let x = 0;
//! x = 1;  // Noncompliant: overwritten on the next line
//! x = 2;
//! ```
//!
//! **Flagged**: when a plain-identifier assignment is immediately overwritten by
//! another assignment to the same name with no intervening statement, and the
//! overwriting RHS does not reference the overwritten variable.  Also flagged: a
//! self-assignment of the form `x = x;`.
//!
//! **Not flagged**:
//! - Read-modify-write on the second line: `x = x + 1`.
//! - Any intervening statement between the two writes.
//! - Non-plain-identifier LHS (member expressions, destructuring).
//! - Augmented assignments (`x += 1`), which are inherently read-modify-write.
//!
//! Behaviour is reproduced from the public RSPEC description (S4165) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{AssignmentTarget, Expression, Statement};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-redundant-assignments";

/// Spans extracted from a plain-identifier write (`=` assignment to a plain
/// identifier).
///
/// All fields are [`Span`] values so the struct is `Copy` and lifetime-free.
#[derive(Clone, Copy)]
struct WriteEntry {
    /// Span of the LHS identifier, e.g. `x` in `x = 1`.
    lhs_span: Span,
    /// Span of the right-hand side expression of the assignment.
    rhs_span: Span,
    /// Span of the entire `ExpressionStatement`. This is the location reported
    /// when the write is determined to be overwritten by the immediately
    /// following statement.
    stmt_span: Span,
}

/// Attempts to extract a [`WriteEntry`] from `stmt`.
///
/// Returns `None` when `stmt` is not a simple `=` assignment to a plain
/// identifier.
fn extract_write(stmt: &Statement<'_>) -> Option<WriteEntry> {
    let Statement::ExpressionStatement(expr_stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &expr_stmt.expression else {
        return None;
    };
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
        return None;
    };
    Some(WriteEntry {
        lhs_span: id.span,
        rhs_span: assign.right.span(),
        stmt_span: expr_stmt.span,
    })
}

impl Scanner<'_> {
    /// Scans `statements` for redundant assignments.
    ///
    /// Two kinds of redundancy are detected:
    ///
    /// 1. **Self-assignment**: a statement `x = x;` where the RHS text equals
    ///    the LHS identifier text.
    ///
    /// 2. **Adjacent dead reassignment**: two consecutive `ExpressionStatement`s
    ///    where both are `=` assignments to the same plain identifier, and the
    ///    second write's RHS does not mention the variable being overwritten
    ///    (read-modify-write guard).  The *first* (dead) assignment is reported.
    pub(crate) fn check_no_redundant_assignments(&mut self, statements: &[Statement<'_>]) {
        let mut prev: Option<WriteEntry> = None;

        for stmt in statements {
            let current = extract_write(stmt);

            // Sub-case 1: self-assignment (`x = x`).
            if let Some(curr_e) = &current {
                let lhs_text = self.text(curr_e.lhs_span);
                let rhs_text = self.text(curr_e.rhs_span);
                if lhs_text == rhs_text {
                    self.report(RULE_NAME, "redundantAssignment", curr_e.stmt_span);
                }
            }

            // Sub-case 2: adjacent dead reassignment.
            if let (Some(prev_e), Some(curr_e)) = (&prev, &current) {
                let lhs_text = self.text(prev_e.lhs_span);
                let same_lhs = lhs_text == self.text(curr_e.lhs_span);
                // Guard: the second write's RHS must not reference the LHS
                // identifier, otherwise it is a read-modify-write.
                let rhs_clear = !self.text(curr_e.rhs_span).contains(lhs_text);
                if same_lhs && rhs_clear {
                    self.report(RULE_NAME, "redundantAssignment", prev_e.stmt_span);
                }
            }

            prev = current;
        }
    }
}
