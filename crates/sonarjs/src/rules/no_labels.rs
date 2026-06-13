//! Rule `no-labels` (SonarJS key S1119).
//!
//! Clean-room port. Labels provide goto-like control flow and hurt readability.
//! This rule flags every `LabeledStatement` unconditionally — including labels
//! placed on loops or switch statements — because SonarJS discourages labels
//! entirely and encourages refactoring to structured control flow instead.
//!
//! ## Detection strategy
//!
//! The hook `visit_labeled_statement` fires for every labeled statement node in
//! the AST. The diagnostic is anchored on `stmt.label.span`, which covers only
//! the identifier part of the label (before the colon), keeping the reported
//! location concise and pointing directly at the label name.
//!
//! ## Syntactic check
//!
//! This is a purely syntactic, unconditional check. All labeled statements are
//! flagged regardless of whether the label is actually referenced by a `break`
//! or `continue` inside the body.
//!
//! Behaviour is reproduced from the public RSPEC S1119 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::LabeledStatement;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-labels";

impl Scanner<'_> {
    pub(crate) fn check_no_labels(&mut self, stmt: &LabeledStatement<'_>) {
        self.report(RULE_NAME, "noLabels", stmt.label.span);
    }
}
