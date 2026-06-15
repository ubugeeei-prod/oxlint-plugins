//! Rule `elseif-without-else` (SonarJS key S126).
//!
//! Clean-room port. An `if … else if …` chain that contains at least one
//! `else if` branch should always end with a final `else` clause. Without a
//! terminal `else`, the developer has not documented (or guarded) the remaining
//! case, which can hide unintentional fall-through behaviour.
//!
//! Only the **head** `if` is reported, and only when:
//! - The chain has at least one `else if` branch (i.e., an `alternate` that is
//!   itself an `IfStatement`), AND
//! - The chain has no terminal `else` (i.e., the last `alternate` is `None`,
//!   not a non-`IfStatement` statement).
//!
//! ## Flagged
//! ```js
//! if (a) {} else if (b) {}
//! if (a) {} else if (b) {} else if (c) {}
//! ```
//!
//! ## Not flagged
//! ```js
//! if (a) {}                          // no else-if at all
//! if (a) {} else {}                  // else but no else-if
//! if (a) {} else if (b) {} else {}   // has terminal else
//! ```
//!
//! Chain-head detection reuses the shared `if_chain_seen` field on
//! `Scanner` (same mechanism used by `no-identical-conditions` and
//! `no-all-duplicated-branches`). Else-if span starts are pushed into the set
//! so the visitor skips them later; duplicate pushes from the other chain rules
//! are harmless because head detection uses `.contains()`.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{IfStatement, Statement};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "elseif-without-else";

impl<'a> Scanner<'a> {
    pub(crate) fn check_elseif_without_else(&mut self, if_stmt: &IfStatement<'a>) {
        // Only process from the chain head (skip else-if members already covered).
        if self.if_chain_seen.contains(&if_stmt.span.start) {
            return;
        }

        // Walk the chain: collect else-if span.starts; track else-if count + terminal else.
        let mut else_if_starts: SmallVec<[u32; 8]> = SmallVec::new();
        let mut else_if_count = 0usize;
        let mut has_terminal_else = false;
        let mut alternate = if_stmt.alternate.as_ref();

        while let Some(stmt) = alternate {
            match stmt {
                Statement::IfStatement(next) => {
                    else_if_count += 1;
                    else_if_starts.push(next.span.start);
                    alternate = next.alternate.as_ref();
                }
                _ => {
                    has_terminal_else = true;
                    break;
                }
            }
        }

        for start in else_if_starts {
            self.if_chain_seen.push(start);
        }

        if else_if_count >= 1 && !has_terminal_else {
            self.report(RULE_NAME, "elseifWithoutElse", if_stmt.span);
        }
    }
}
