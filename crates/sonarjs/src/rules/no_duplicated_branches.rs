//! Rule `no-duplicated-branches` (SonarJS key S1871).
//!
//! Clean-room port. When ANY two branches in an if/else-if/else chain, or any
//! two case/default clauses in a switch statement, have byte-identical
//! implementations, the second (duplicate) branch is flagged.
//!
//! This differs from `no-all-duplicated-branches` (S3923) which only fires
//! when ALL branches are identical and a terminal else/default is present.
//!
//! Two structures are covered:
//!
//! ## A. if / else-if / else chains
//!
//! The head `if` is the entry point. For each subsequent branch (else-if
//! consequent or terminal else body) whose source text matches any earlier
//! branch in the same chain, a diagnostic is produced on the duplicate branch
//! body's span. Only non-empty branches (with at least one statement) are
//! compared; bare empty-block branches are skipped.
//!
//! Head detection reuses the shared `if_chain_seen` set (same one used by
//! `no-identical-conditions` and `no-all-duplicated-branches`): at entry each
//! check function skips nodes whose `span.start` is already in the set, then
//! pushes each else-if's `span.start` so it is skipped on the visitor's
//! subsequent visit.
//!
//! ## B. switch statements
//!
//! For each case clause whose body (the span from first to last consequent
//! statement) is byte-identical to any earlier case clause with a non-empty
//! body, a diagnostic is produced on the duplicate body's span. Fall-through
//! cases (with zero consequent statements) are skipped.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{IfStatement, Statement, SwitchStatement};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-duplicated-branches";

/// Returns `true` when the statement has at least one body statement. An
/// empty `BlockStatement` is considered empty; every other kind of statement
/// is considered non-empty.
fn is_nonempty_branch(stmt: &Statement) -> bool {
    match stmt {
        Statement::BlockStatement(block) => !block.body.is_empty(),
        _ => true,
    }
}

impl<'a> Scanner<'a> {
    /// Check rule A: if / else-if / else chains.
    pub(crate) fn check_no_duplicated_branches_if(&mut self, if_stmt: &IfStatement<'a>) {
        // Skip else-if nodes already processed as part of their chain head.
        if self.if_chain_seen.contains(&if_stmt.span.start) {
            return;
        }

        // Phase 1 — immutable pass: walk the chain, accumulating seen branch
        // texts and detecting duplicates.
        let mut seen: SmallVec<[&'a str; 16]> = SmallVec::new();
        let mut else_if_starts: SmallVec<[u32; 16]> = SmallVec::new();
        let mut report_spans: SmallVec<[Span; 8]> = SmallVec::new();

        // Head branch — only add to `seen`; never report the first occurrence.
        if is_nonempty_branch(&if_stmt.consequent) {
            seen.push(self.text(if_stmt.consequent.span()));
        }

        let mut alternate = if_stmt.alternate.as_ref();

        while let Some(stmt) = alternate {
            match stmt {
                Statement::IfStatement(next) => {
                    else_if_starts.push(next.span.start);
                    if is_nonempty_branch(&next.consequent) {
                        let text = self.text(next.consequent.span());
                        if seen.contains(&text) {
                            report_spans.push(next.consequent.span());
                        }
                        seen.push(text);
                    }
                    alternate = next.alternate.as_ref();
                }
                other => {
                    if is_nonempty_branch(other) {
                        let text = self.text(other.span());
                        if seen.contains(&text) {
                            report_spans.push(other.span());
                        }
                    }
                    break;
                }
            }
        }

        // Phase 2 — mutable pass: record else-if starts so the visitor skips
        // them later (shared with no-identical-conditions and
        // no-all-duplicated-branches; harmless duplicates).
        for start in else_if_starts {
            self.if_chain_seen.push(start);
        }

        // Phase 3 — report duplicate branches.
        for span in report_spans {
            self.report(RULE_NAME, "duplicatedBranch", span);
        }
    }

    /// Check rule B: switch statements.
    pub(crate) fn check_no_duplicated_branches_switch(
        &mut self,
        switch_stmt: &SwitchStatement<'a>,
    ) {
        let cases = &switch_stmt.cases;
        if cases.len() < 2 {
            return;
        }

        // Phase 1 — immutable pass: collect non-empty case body texts.
        let mut seen: SmallVec<[&'a str; 16]> = SmallVec::new();
        let mut report_spans: SmallVec<[Span; 8]> = SmallVec::new();

        for case in cases {
            match (case.consequent.first(), case.consequent.last()) {
                (Some(first), Some(last)) => {
                    let span = Span::new(first.span().start, last.span().end);
                    let text = self.text(span);
                    if seen.contains(&text) {
                        report_spans.push(span);
                    }
                    seen.push(text);
                }
                _ => {
                    // Fall-through case (empty consequent) — skip.
                }
            }
        }

        // Phase 2 — report duplicate cases.
        for span in report_spans {
            self.report(RULE_NAME, "duplicatedBranch", span);
        }
    }
}
