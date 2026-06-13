//! Rule `no-identical-conditions` (SonarJS key S1862).
//!
//! Clean-room port. Within a single `if / else if / else if …` chain, two
//! branches must not test the same condition. A later branch whose condition
//! is textually identical to an earlier one in the same chain is dead code
//! because the first matching branch will always be taken first.
//!
//! Scope is strictly one chain: the head `if` plus any `else if` branches
//! that follow it directly. A nested `if` inside a branch body is a separate
//! chain and is evaluated independently. Only the later (duplicate) condition
//! span is reported; the first occurrence is left alone.
//!
//! Conditions are compared by their source text (via `Scanner::text`), which
//! is the same strategy used by `no-duplicate-in-composite`. This is a
//! best-effort syntactic check; semantically equivalent but textually distinct
//! conditions are not detected.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{IfStatement, Statement};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-identical-conditions";

impl<'a> Scanner<'a> {
    pub(crate) fn check_no_identical_conditions(&mut self, if_stmt: &IfStatement<'a>) {
        // Skip else-if nodes that were already processed as part of their head's chain.
        if self.if_chain_seen.contains(&if_stmt.span.start) {
            return;
        }

        // Treat this node as the chain head.  Walk the chain following
        // `alternate` while it is itself an IfStatement (i.e. `else if`).
        //
        // Phase 1 — immutable pass: collect (condition_text, condition_span)
        // pairs for every branch in the chain, and record the span.start of
        // every else-if node so they can be skipped later.
        let mut conditions: SmallVec<[(&'a str, Span); 8]> = SmallVec::new();
        let mut else_if_starts: SmallVec<[u32; 8]> = SmallVec::new();

        conditions.push((self.text(if_stmt.test.span()), if_stmt.test.span()));

        let mut alternate = if_stmt.alternate.as_ref();
        while let Some(Statement::IfStatement(next)) = alternate {
            else_if_starts.push(next.span.start);
            conditions.push((self.text(next.test.span()), next.test.span()));
            alternate = next.alternate.as_ref();
        }

        // Phase 2 — mutable pass: record else-if starts so they are skipped
        // when the visitor reaches them later.
        for start in else_if_starts {
            self.if_chain_seen.push(start);
        }

        // Phase 3 — detect duplicates and collect spans to report.
        let mut seen_texts: SmallVec<[&'a str; 8]> = SmallVec::new();
        let mut duplicates: SmallVec<[Span; 4]> = SmallVec::new();
        for (text, span) in &conditions {
            if seen_texts.contains(text) {
                duplicates.push(*span);
            } else {
                seen_texts.push(*text);
            }
        }

        // Phase 4 — report.
        for span in duplicates {
            self.report(RULE_NAME, "identicalConditions", span);
        }
    }
}
