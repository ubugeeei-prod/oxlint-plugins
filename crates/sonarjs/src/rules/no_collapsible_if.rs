//! Rule `no-collapsible-if` (SonarJS key S1066).
//!
//! Clean-room port. Reports an outer `if` statement whose sole purpose is to
//! contain a single nested `if` statement, signalling that the two conditions
//! should be merged with `&&` to reduce unnecessary nesting. Behaviour is
//! reproduced from the public RSPEC description only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.
//!
//! Semantics: the outer `if` is flagged when (1) it has no `else` clause,
//! (2) its consequent is either directly an inner `if` statement or a block
//! containing exactly one `if` statement, and (3) that inner `if` also has no
//! `else` clause. The diagnostic is reported on the outer `if` keyword only.

use oxc_ast::ast::{IfStatement, Statement};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-collapsible-if";

/// Returns the inner `IfStatement` when `consequent` is collapsible, i.e.
/// when it is either a direct `if` or a single-statement block containing an
/// `if`. Returns `None` otherwise.
fn inner_collapsible_if<'a, 'b>(consequent: &'b Statement<'a>) -> Option<&'b IfStatement<'a>> {
    match consequent {
        Statement::IfStatement(inner) => Some(inner),
        Statement::BlockStatement(block) if block.body.len() == 1 => match &block.body[0] {
            Statement::IfStatement(inner) => Some(inner),
            _ => None,
        },
        _ => None,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_collapsible_if(&mut self, if_stmt: &IfStatement<'_>) {
        if if_stmt.alternate.is_some() {
            return;
        }
        let Some(inner) = inner_collapsible_if(&if_stmt.consequent) else {
            return;
        };
        if inner.alternate.is_some() {
            return;
        }
        let start = if_stmt.span.start;
        let keyword = Span::new(start, start + 2);
        self.report(RULE_NAME, "collapsibleIf", keyword);
    }
}
