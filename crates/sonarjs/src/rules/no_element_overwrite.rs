//! Rule `no-element-overwrite` (SonarJS key S4143).
//!
//! Clean-room port. Writing to the same collection key or index twice in a row,
//! with no read of that element in between, means the first write is dead—almost
//! always a bug (wrong index/key).
//!
//! ```js
//! var a = [];
//! a[0] = 1;  // Noncompliant: overwritten on the next line
//! a[0] = 2;
//! ```
//!
//! **Flagged**: when two consecutive `ExpressionStatement`s in the same
//! statement list are both simple `=` assignments to the same collection
//! element, with no intervening statement, and the second assignment's
//! right-hand side does not reference the element being written.
//!
//! Recognised write forms:
//! - Indexed writes with a numeric or string literal key: `arr[0] = …` / `m["k"] = …`
//! - Static property writes: `obj.prop = …`
//!
//! **Not flagged**:
//! - Different receiver, different key/prop, or a non-literal computed key
//!   (variable index — skipped for safety).
//! - Read-modify-write on the second line: `a[0] = a[0] + 1`.
//! - Any non-plain-identifier receiver (chained access like `foo.bar[0] = …`).
//! - Non-consecutive writes (any intervening statement breaks the pair).
//!
//! Behaviour is reproduced from the public RSPEC description (S4143) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{AssignmentTarget, Expression, Statement};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-element-overwrite";

/// Spans extracted from a collection-element write (`=` assignment to a member
/// expression with a plain-identifier receiver and a literal or static key).
///
/// All fields are [`Span`] values so the struct is `Copy` and lifetime-free.
#[derive(Clone, Copy)]
struct WriteEntry {
    /// Span of the receiver identifier, e.g. `arr` in `arr[0]` or `obj` in `obj.x`.
    receiver_span: Span,
    /// Span of the key text used for equality comparison:
    /// — for computed writes: the span of the numeric/string literal (e.g. `0`)
    /// — for static writes: the span of the property `IdentifierName` (e.g. `x`)
    key_span: Span,
    /// Span of the whole LHS member expression, e.g. `arr[0]` or `obj.x`.
    /// Used as the substring to look for in the second write's RHS to detect
    /// read-modify-write patterns such as `a[0] = a[0] + 1`.
    lhs_span: Span,
    /// Span of the right-hand side expression of the assignment.
    rhs_span: Span,
    /// Span of the entire `ExpressionStatement`.  This is the location reported
    /// when the write is determined to be overwritten by the immediately
    /// following statement.
    stmt_span: Span,
}

/// Attempts to extract a [`WriteEntry`] from `stmt`.
///
/// Returns `None` when `stmt` is not a simple `=` assignment to a member
/// expression with a plain-identifier receiver and a static-property or
/// literal-computed key.
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

    match &assign.left {
        AssignmentTarget::ComputedMemberExpression(member) => {
            // Receiver must be a plain identifier (no chained access).
            let Expression::Identifier(id) = &member.object else {
                return None;
            };
            // Key must be a numeric or string literal for zero-FP detection.
            let key_span = match &member.expression {
                Expression::NumericLiteral(n) => n.span,
                Expression::StringLiteral(s) => s.span,
                _ => return None,
            };
            Some(WriteEntry {
                receiver_span: id.span,
                key_span,
                lhs_span: member.span,
                rhs_span: assign.right.span(),
                stmt_span: expr_stmt.span,
            })
        }
        AssignmentTarget::StaticMemberExpression(member) => {
            // Receiver must be a plain identifier (no chained access).
            let Expression::Identifier(id) = &member.object else {
                return None;
            };
            Some(WriteEntry {
                receiver_span: id.span,
                // Use the property IdentifierName span so comparison is by text.
                key_span: member.property.span,
                lhs_span: member.span,
                rhs_span: assign.right.span(),
                stmt_span: expr_stmt.span,
            })
        }
        _ => None,
    }
}

impl Scanner<'_> {
    /// Scans `statements` for pairs of consecutive element overwrites.
    ///
    /// Two adjacent `ExpressionStatement`s are flagged when:
    /// 1. Both are simple `=` assignments to the same collection element
    ///    (same plain-identifier receiver **and** same literal/static key).
    /// 2. The second write's right-hand side does not contain the element text
    ///    (to exclude read-modify-write patterns like `a[0] = a[0] + 1`).
    ///
    /// The *first* (overwritten) assignment is reported.
    pub(crate) fn check_no_element_overwrite(&mut self, statements: &[Statement<'_>]) {
        let mut prev: Option<WriteEntry> = None;

        for stmt in statements {
            let current = extract_write(stmt);

            if let (Some(prev_e), Some(curr_e)) = (&prev, &current) {
                let receiver_eq =
                    self.text(prev_e.receiver_span) == self.text(curr_e.receiver_span);
                let key_eq = self.text(prev_e.key_span) == self.text(curr_e.key_span);
                // Guard: the second write's RHS must not contain the LHS element
                // text, otherwise it is a read-modify-write, not an overwrite.
                let rhs_clear = !self
                    .text(curr_e.rhs_span)
                    .contains(self.text(curr_e.lhs_span));
                if receiver_eq && key_eq && rhs_clear {
                    self.report(RULE_NAME, "elementOverwrite", prev_e.stmt_span);
                }
            }

            prev = current;
        }
    }
}
