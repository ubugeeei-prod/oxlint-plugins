//! Rule `no-invariant-returns` (SonarJS key S3516).
//!
//! Clean-room port. A function that always returns the same value regardless
//! of its logic provides no useful information to callers; the return value is
//! de facto a constant. Such functions are almost always a bug — either a
//! missing branch, a forgotten side-effect, or a stale copy-paste.
//!
//! ## Detection strategy
//!
//! A stack of frames (one per open function/arrow scope) accumulates the
//! source-text of every value-returning `return <expr>;` statement, and a
//! flag for whether any bare `return;` was seen. When the frame is popped:
//!
//! - At least two value returns must have been seen.
//! - No bare `return;` must have been seen.
//! - Every collected return expression must be byte-identical (compared as
//!   raw source text via `self.text(span)`).
//!
//! Expression-bodied arrows (`() => expr`) contain no `return` statement at
//! the AST level, so they are never flagged.
//!
//! ```js
//! function always42(x) {
//!   if (x > 0) return 42;  // ← same value
//!   return 42;              // ← same value → function is flagged
//! }
//! ```
//!
//! **Flagged**: a function whose every explicit value return yields the same
//! source-text expression, with at least two such returns and no bare return.
//!
//! **Not flagged**:
//! - functions with only one value return;
//! - functions that mix value returns with bare `return;`;
//! - functions whose value returns differ;
//! - expression-bodied arrow functions.
//!
//! Behaviour is derived from the public RSPEC description (S3516) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::{InvariantReturnFrame, Scanner};

pub(crate) const RULE_NAME: &str = "no-invariant-returns";

impl<'a> Scanner<'a> {
    /// Pushes a tracking frame when entering a function or arrow scope.
    pub(crate) fn enter_invariant_return_scope(&mut self, span: Span) {
        self.invariant_return_stack.push(InvariantReturnFrame {
            span,
            return_values: SmallVec::new(),
            has_bare_return: false,
        });
    }

    /// Records a return statement in the innermost open scope.
    /// `value_text` is `Some(text)` for `return <expr>;` and `None` for a
    /// bare `return;`.
    pub(crate) fn record_invariant_return(&mut self, value_text: Option<&'a str>) {
        let Some(frame) = self.invariant_return_stack.last_mut() else {
            return;
        };
        match value_text {
            None => {
                frame.has_bare_return = true;
            }
            Some(text) => {
                frame.return_values.push(text);
            }
        }
    }

    /// Pops the innermost frame and reports the scope when every explicit
    /// value return yields the same source-text expression (at least two
    /// such returns, no bare return).
    pub(crate) fn leave_invariant_return_scope(&mut self) {
        let Some(frame) = self.invariant_return_stack.pop() else {
            return;
        };
        if frame.has_bare_return || frame.return_values.len() < 2 {
            return;
        }
        let first = frame.return_values[0];
        if frame.return_values.iter().all(|&v| v == first) {
            self.report(RULE_NAME, "invariantReturn", frame.span);
        }
    }
}
