//! Rule `no-fallthrough` (SonarJS key S128).
//!
//! Clean-room port. A non-empty `switch` case should make its control flow
//! explicit before the next case starts. This rule reports a case that can
//! continue into the following case unless it ends with an unconditional jump
//! (`break`, `return`, `throw`, or `continue`) or has an intentional
//! fallthrough comment between the last statement and the next case label.
//!
//! The control-flow check is intentionally conservative and local: nested
//! functions are not inspected, and complex statements only count as
//! terminating when every visible branch is terminating (for example,
//! `if (...) return; else throw e;`). Labeled `break` is not treated as
//! terminating because, without resolving the label target, it may only exit a
//! local labeled block.
//!
//! Behaviour is reproduced from the public RSPEC S128 description and ESLint's
//! documented fallthrough-comment convention only; no upstream source, tests,
//! fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Statement, SwitchCase, SwitchStatement, TryStatement};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-fallthrough";

fn statement_terminates(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::BreakStatement(brk) => brk.label.is_none(),
        Statement::ContinueStatement(_)
        | Statement::ReturnStatement(_)
        | Statement::ThrowStatement(_) => true,
        Statement::BlockStatement(block) => block.body.last().is_some_and(statement_terminates),
        Statement::LabeledStatement(label) => statement_terminates(&label.body),
        Statement::IfStatement(if_stmt) => {
            let Some(alternate) = &if_stmt.alternate else {
                return false;
            };
            statement_terminates(&if_stmt.consequent) && statement_terminates(alternate)
        }
        Statement::TryStatement(try_stmt) => try_statement_terminates(try_stmt.as_ref()),
        _ => false,
    }
}

fn try_statement_terminates(stmt: &TryStatement<'_>) -> bool {
    if stmt
        .finalizer
        .as_ref()
        .is_some_and(|finalizer| finalizer.body.last().is_some_and(statement_terminates))
    {
        return true;
    }

    let block_terminates = stmt.block.body.last().is_some_and(statement_terminates);
    let Some(handler) = &stmt.handler else {
        return block_terminates;
    };
    block_terminates && handler.body.body.last().is_some_and(statement_terminates)
}

fn case_label_span(case: &SwitchCase<'_>) -> Span {
    let len = if case.test.is_some() { 4 } else { 7 };
    Span::new(case.span.start, case.span.start + len)
}

fn fallthrough_comment_text(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    normalized.contains("fall through")
        || normalized.contains("falls through")
        || normalized.contains("fallthrough")
}

impl Scanner<'_> {
    pub(crate) fn check_no_fallthrough(&mut self, switch_stmt: &SwitchStatement<'_>) {
        let mut spans: SmallVec<[Span; 4]> = SmallVec::new();

        for pair in switch_stmt.cases.windows(2) {
            let current = &pair[0];
            let next = &pair[1];
            let Some(last_stmt) = current.consequent.last() else {
                continue;
            };
            if statement_terminates(last_stmt) {
                continue;
            }
            if self.has_fallthrough_comment(last_stmt.span().end, next.span.start) {
                continue;
            }
            spans.push(case_label_span(current));
        }

        for span in spans {
            self.report(RULE_NAME, "noFallthrough", span);
        }
    }

    fn has_fallthrough_comment(&self, start: u32, end: u32) -> bool {
        self.comment_spans.iter().any(|span| {
            span.start >= start && span.end <= end && fallthrough_comment_text(self.text(*span))
        })
    }
}
