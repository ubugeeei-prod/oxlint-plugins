//! Rule `no-unenclosed-multiline-block` (SonarJS key S2681).
//!
//! Clean-room port. When an unbraced control-structure body (`if`/`for`/`while`/
//! `else` without `{}`) is a single statement, and the next sibling statement in
//! the enclosing block is indented as if it also belongs to the body, the layout
//! is misleading.
//!
//! ```js
//! if (cond)
//!   doFirst();
//!   doSecond(); // Noncompliant: looks guarded by the if, but always runs
//! ```
//!
//! **Flagged**: the next sibling of an unbraced control structure (if/for/while)
//! when ALL of these hold:
//!   - the sibling starts on a later line than the body statement
//!   - the sibling's start column is strictly greater than the control keyword's
//!     start column (it is more indented than the `if`/`for`/`while`)
//!   - the sibling's start column is at least as great as the body statement's
//!     start column (it is indented at least as much as the body)
//!
//! **Not flagged**:
//!   - braced bodies — `if (c) { ... }` is unambiguous
//!   - `else if` chains — the alternate `IfStatement` is not a misleading sibling
//!   - siblings that start at or before the control keyword's column
//!   - siblings that start to the left of the body statement
//!
//! Behaviour is reproduced from the public RSPEC description (S2681) only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{IfStatement, Statement};
use oxc_span::{GetSpan, Span};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-unenclosed-multiline-block";

impl Scanner<'_> {
    pub(crate) fn check_no_unenclosed_multiline_block(&mut self, statements: &[Statement<'_>]) {
        for (i, statement) in statements.iter().enumerate() {
            match statement {
                Statement::IfStatement(if_stmt) => {
                    self.check_unenclosed_if(i, statements, if_stmt);
                }
                Statement::ForStatement(for_stmt) => {
                    self.check_unenclosed_body(i, statements, for_stmt.span, &for_stmt.body);
                }
                Statement::ForInStatement(for_in) => {
                    self.check_unenclosed_body(i, statements, for_in.span, &for_in.body);
                }
                Statement::ForOfStatement(for_of) => {
                    self.check_unenclosed_body(i, statements, for_of.span, &for_of.body);
                }
                Statement::WhileStatement(while_stmt) => {
                    self.check_unenclosed_body(i, statements, while_stmt.span, &while_stmt.body);
                }
                _ => {}
            }
        }
    }

    fn check_unenclosed_if(
        &mut self,
        i: usize,
        statements: &[Statement<'_>],
        if_stmt: &IfStatement<'_>,
    ) {
        self.check_unenclosed_body(i, statements, if_stmt.span, &if_stmt.consequent);

        let Some(alternate) = &if_stmt.alternate else {
            return;
        };
        if matches!(
            alternate,
            Statement::BlockStatement(_) | Statement::IfStatement(_)
        ) {
            return;
        }
        self.flag_if_misleading(if_stmt.span, alternate.span(), i, statements);
    }

    fn check_unenclosed_body(
        &mut self,
        i: usize,
        statements: &[Statement<'_>],
        ctrl_span: Span,
        body: &Statement<'_>,
    ) {
        if matches!(body, Statement::BlockStatement(_)) {
            return;
        }
        self.flag_if_misleading(ctrl_span, body.span(), i, statements);
    }

    fn flag_if_misleading(
        &mut self,
        ctrl_span: Span,
        body_span: Span,
        i: usize,
        statements: &[Statement<'_>],
    ) {
        let Some(next) = statements.get(i + 1) else {
            return;
        };
        let next_span = next.span();
        let ctrl_loc = self.line_index.loc_for_span(self.source_text, ctrl_span);
        let body_loc = self.line_index.loc_for_span(self.source_text, body_span);
        let next_loc = self.line_index.loc_for_span(self.source_text, next_span);

        if next_loc.start_line <= body_loc.start_line {
            return;
        }
        if next_loc.start_column <= ctrl_loc.start_column {
            return;
        }
        if next_loc.start_column < body_loc.start_column {
            return;
        }
        self.report(RULE_NAME, "unenclosedMultilineBlock", next_span);
    }
}
