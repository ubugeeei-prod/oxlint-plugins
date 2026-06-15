//! Rule `label-position` (SonarJS key S1439).
//!
//! Clean-room port. Labels are only useful when attached directly to a loop or
//! switch statement, where `break` or `continue` can target the labelled
//! breakable construct. Labels on other statements add indirection without
//! enabling structured control flow.
//!
//! ## Detection strategy
//!
//! The `visit_labeled_statement` hook sees every labelled statement. This rule
//! inspects the labelled statement's direct body and reports the label when the
//! body is not a loop or `switch`.
//!
//! Nested labels intentionally report the outer label when its direct body is
//! another labelled statement, even if the inner label is attached to a loop.
//! The label must be directly on the loop or switch to be accepted.

use oxc_ast::ast::{LabeledStatement, Statement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "label-position";

impl Scanner<'_> {
    pub(crate) fn check_label_position(&mut self, stmt: &LabeledStatement<'_>) {
        if is_allowed_label_body(&stmt.body) {
            return;
        }

        self.report(RULE_NAME, "removeLabel", stmt.label.span);
    }
}

fn is_allowed_label_body(stmt: &Statement<'_>) -> bool {
    matches!(
        stmt,
        Statement::ForStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::SwitchStatement(_)
    )
}
