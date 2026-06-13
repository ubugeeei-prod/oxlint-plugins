//! Rule `no-all-duplicated-branches` (SonarJS key S3923).
//!
//! Clean-room port. A conditional structure whose every branch has the exact
//! same implementation is pointless: the condition has no effect on the
//! program's output, and the duplication most likely signals a mistake.
//!
//! Two structures are covered:
//!
//! ## A. if / else-if / else chains
//!
//! The head `if` is reported when ALL of the following hold:
//! - The chain has a terminal `else` (the last `alternate` is a non-`if`
//!   `Statement`, e.g. a block or a bare statement — NOT `None` and NOT
//!   another `IfStatement`).
//! - The chain has at least 2 branches total (head consequent + at least one
//!   else-if consequent or the final else body).
//! - Every branch body (head consequent, each else-if consequent, and the
//!   terminal else body) has identical source text, compared via
//!   `Scanner::text(stmt.span())`.
//!
//! Head detection reuses the shared `if_chain_seen` set (same one used by
//! `no-identical-conditions`): at entry each check function skips nodes whose
//! `span.start` is already in the set, then pushes each else-if's `span.start`
//! so it is skipped on the visitor's subsequent visit. Sharing is intentional
//! and safe: both rules skip else-if members, and duplicate pushes are harmless.
//!
//! ## B. switch statements
//!
//! The `switch` is reported when ALL of the following hold:
//! - The switch has a `default` case (a `SwitchCase` with `test: None`).
//! - It has at least 2 cases total (including `default`).
//! - Every case's body has identical source text. A case body is defined as the
//!   source spanning `first_consequent.span().start .. last_consequent.span().end`
//!   when the case has ≥1 consequent statement; empty-string `""` when the case
//!   has 0 consequent statements (fall-through case).
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{IfStatement, Statement, SwitchStatement};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-all-duplicated-branches";

/// Returns `true` iff `texts` has at least two elements and every element
/// equals the first.
fn all_equal(texts: &[&str]) -> bool {
    texts.len() >= 2 && texts.iter().all(|t| *t == texts[0])
}

impl<'a> Scanner<'a> {
    /// Check rule A: if / else-if / else chains.
    pub(crate) fn check_no_all_duplicated_branches_if(&mut self, if_stmt: &IfStatement<'a>) {
        // Skip else-if nodes already processed as part of their chain head.
        if self.if_chain_seen.contains(&if_stmt.span.start) {
            return;
        }

        // Phase 1 — immutable pass: walk the chain, collecting branch-body
        // texts and the span.start of each else-if node.
        let mut bodies: SmallVec<[&'a str; 8]> = SmallVec::new();
        let mut else_if_starts: SmallVec<[u32; 8]> = SmallVec::new();

        bodies.push(self.text(if_stmt.consequent.span()));

        let mut alternate = if_stmt.alternate.as_ref();
        let mut has_terminal_else = false;

        while let Some(stmt) = alternate {
            match stmt {
                Statement::IfStatement(next) => {
                    else_if_starts.push(next.span.start);
                    bodies.push(self.text(next.consequent.span()));
                    alternate = next.alternate.as_ref();
                }
                other => {
                    // Terminal else: collect its text and mark the flag.
                    bodies.push(self.text(other.span()));
                    has_terminal_else = true;
                    break;
                }
            }
        }

        // Phase 2 — mutable pass: record else-if starts so the visitor skips
        // them later (shared with no-identical-conditions; harmless duplicates).
        for start in else_if_starts {
            self.if_chain_seen.push(start);
        }

        // Report only when there is a terminal else and every body is identical.
        if has_terminal_else && all_equal(&bodies) {
            self.report(RULE_NAME, "allDuplicatedBranches", if_stmt.span);
        }
    }

    /// Check rule B: switch statements.
    pub(crate) fn check_no_all_duplicated_branches_switch(
        &mut self,
        switch_stmt: &SwitchStatement<'a>,
    ) {
        let cases = &switch_stmt.cases;

        // Need at least 2 cases and a default.
        if cases.len() < 2 {
            return;
        }
        let has_default = cases.iter().any(|c| c.test.is_none());
        if !has_default {
            return;
        }

        // Phase 1 — immutable pass: collect case body texts.
        let mut bodies: SmallVec<[&'a str; 8]> = SmallVec::new();
        for case in cases {
            let text = match (case.consequent.first(), case.consequent.last()) {
                (Some(first), Some(last)) => {
                    self.text(Span::new(first.span().start, last.span().end))
                }
                _ => "",
            };
            bodies.push(text);
        }

        // Phase 2 — report if all bodies are identical.
        if all_equal(&bodies) {
            self.report(RULE_NAME, "allDuplicatedBranches", switch_stmt.span);
        }
    }
}
