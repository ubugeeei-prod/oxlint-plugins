//! Rule `updated-loop-counter` (SonarJS key S2310).
//!
//! Clean-room port. A classic `for` loop's counter variable should only be
//! advanced by the loop's update clause. Reassigning or mutating that counter
//! inside the loop *body* makes the control flow hard to follow and is a
//! frequent source of bugs (off-by-one errors, accidental infinite loops). The
//! rule flags an assignment (`i = …`, `i += 1`) or an increment/decrement
//! (`i++`, `--i`) to a loop counter that occurs anywhere inside that loop's
//! body.
//!
//! The "loop counter(s)" are the variables modified by the for-statement's
//! UPDATE clause: the `i` in `for (…; …; i++)`, in `for (…; …; i += 2)`, and
//! each variable of a comma-separated update such as `for (…; …; i++, j++)`.
//! The update clause's own write is never flagged — only writes located inside
//! the loop body are.
//!
//! ## Scope
//!
//! - Only classic `for (init; test; update)` loops are covered. `for-in` and
//!   `for-of` loops have no update clause and therefore no counter in this
//!   sense; reassigning their loop variable is handled by
//!   `no-parameter-reassignment` (S1226), not here.
//! - Only the *binding itself* is flagged. A property write (`i.x = 1`,
//!   `i[0] = 1`) leaves the counter binding intact and is never reported —
//!   those targets are member expressions, not the bare identifier.
//!
//! Resolution relies on semantic analysis and compares SYMBOLS, not names, so a
//! body assignment to a *shadowing* local that merely reuses the counter's name
//! (`for (let i = 0; …; i++) { let i = 0; i = 5; }`) is correctly left
//! unflagged. When semantic information is unavailable or a reference cannot be
//! resolved, nothing is reported (conservative — no false positives).
//!
//! Behaviour is reproduced from the public RSPEC description (S2310) and the
//! observable behaviour of the equivalent check only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `for (let i = 0; i < 10; i++) { i = 5; }`
//! - `for (let i = 0; i < 10; i++) { i += 2; }`
//! - `for (let i = 0; i < 10; i++) { if (x) i--; }`
//!
//! ## Not flagged
//! - `for (let i = 0; i < 10; i++) {}` — counter touched only by the update
//! - `for (let i = 0; i < 10; i++) { let i = 0; i = 5; }` — inner `i` shadows
//! - `for (let i = 0; i < 10; i++) { j = 5; }` — a different variable
//! - `for (const x of xs) { x = 1; }` — for-of variable, not a for-counter

use oxc_ast::ast::{
    AssignmentTarget, Expression, ForStatement, IdentifierReference, SimpleAssignmentTarget,
};
use oxc_semantic::SymbolId;
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::{LoopCounterFrame, Scanner};

pub(crate) const RULE_NAME: &str = "updated-loop-counter";

impl<'a> Scanner<'a> {
    /// Resolves an identifier use to the symbol it refers to, or `None` when
    /// semantic analysis is absent or the reference cannot be resolved.
    fn reference_symbol_id(&self, ident: &IdentifierReference<'a>) -> Option<SymbolId> {
        let scoping = self.scoping?;
        let reference_id = ident.reference_id.get()?;
        scoping.get_reference(reference_id).symbol_id()
    }

    /// Collects the counter symbol(s) named by a for-statement update clause
    /// into `out`: an `UpdateExpression` (`i++`), an `AssignmentExpression`
    /// (`i += 1`), or each sub-expression of a `SequenceExpression`
    /// (`i++, j++`). Unresolvable references are skipped.
    fn collect_counter_symbols(&self, expr: &Expression<'a>, out: &mut SmallVec<[SymbolId; 2]>) {
        match expr {
            Expression::UpdateExpression(update) => {
                let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) = &update.argument
                else {
                    return;
                };
                let Some(symbol_id) = self.reference_symbol_id(ident) else {
                    return;
                };
                out.push(symbol_id);
            }
            Expression::AssignmentExpression(assign) => {
                let AssignmentTarget::AssignmentTargetIdentifier(ident) = &assign.left else {
                    return;
                };
                let Some(symbol_id) = self.reference_symbol_id(ident) else {
                    return;
                };
                out.push(symbol_id);
            }
            Expression::SequenceExpression(seq) => {
                for sub in &seq.expressions {
                    self.collect_counter_symbols(sub, out);
                }
            }
            _ => {}
        }
    }

    /// Pushes a loop-counter frame for `stmt` before its body is walked.
    /// Returns `true` when a frame was pushed (so the caller knows to pop it).
    /// No frame is pushed when the loop has no update clause or no update-clause
    /// reference resolves to a symbol.
    pub(crate) fn enter_updated_loop_counter(&mut self, stmt: &ForStatement<'a>) -> bool {
        let Some(update) = &stmt.update else {
            return false;
        };
        let mut counters = SmallVec::new();
        self.collect_counter_symbols(update, &mut counters);
        if counters.is_empty() {
            return false;
        }
        self.loop_counter_symbols.push(LoopCounterFrame {
            counters,
            update_span: update.span(),
        });
        true
    }

    /// Pops the loop-counter frame pushed by [`Self::enter_updated_loop_counter`]
    /// once the loop body has been fully walked.
    pub(crate) fn leave_updated_loop_counter(&mut self, pushed: bool) {
        if pushed {
            self.loop_counter_symbols.pop();
        }
    }

    /// Reports a bare-identifier write (`i = …`, `i += 1`, `i++`, `--i`) whose
    /// target resolves to a counter of any enclosing `for` loop, unless the
    /// write is the loop's own update clause (identified by span containment).
    pub(crate) fn check_updated_loop_counter(
        &mut self,
        ident: &IdentifierReference<'a>,
        span: Span,
    ) {
        if self.loop_counter_symbols.is_empty() {
            return;
        }
        let Some(symbol_id) = self.reference_symbol_id(ident) else {
            return;
        };
        let mut in_update_clause = false;
        let mut matches_counter = false;
        for frame in &self.loop_counter_symbols {
            if frame.update_span.start <= span.start && span.end <= frame.update_span.end {
                in_update_clause = true;
            }
            if frame.counters.contains(&symbol_id) {
                matches_counter = true;
            }
        }
        if matches_counter && !in_update_clause {
            self.report(RULE_NAME, "noCounterUpdate", span);
        }
    }
}
