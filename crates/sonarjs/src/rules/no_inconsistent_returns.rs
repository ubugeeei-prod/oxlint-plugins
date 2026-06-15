//! Rule `no-inconsistent-returns` (SonarJS key S3801).
//!
//! Clean-room port. A function that sometimes returns a value (`return x;`) and
//! sometimes returns nothing (`return;`) is confusing: a caller cannot tell from
//! one `return` whether the function is meant to produce a value, and the bare
//! `return` silently yields `undefined`. Such a function should return a value
//! on every path or on none.
//!
//! ## Stack-based return tracking
//!
//! A `return` statement always binds to the **nearest enclosing function or
//! arrow** — it can never cross a function boundary. A stack of frames (one per
//! open function/arrow scope) therefore attributes each `return` to the correct
//! innermost scope. Each frame records whether the scope has seen an explicit
//! value return and whether it has seen an explicit bare return; when the frame
//! is popped, the scope is reported if **both** were seen.
//!
//! ```js
//! function f(x) {
//!   if (!x) return;      // bare return
//!   return x.value;      // value return → f is flagged
//! }
//! ```
//!
//! **Flagged**: a function/arrow whose body contains at least one `return x;`
//! and at least one bare `return;`.
//!
//! **Not flagged**:
//! - a scope that only ever returns values, or only ever returns nothing;
//! - returns belonging to a nested function/arrow (tracked on their own frame).
//!
//! Narrow form: only the mix of two *explicit* return statements is reported;
//! the case of an explicit value return combined with an implicit fall-through
//! (no trailing `return`) is a documented follow-up. Behaviour is reproduced
//! from the public RSPEC description (S3801) only; no upstream source, tests,
//! fixtures, or message strings were consulted or copied.

use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-inconsistent-returns";

impl Scanner<'_> {
    /// Pushes a tracking frame when entering a function or arrow scope.
    pub(crate) fn enter_return_scope(&mut self, span: Span) {
        self.return_kind_stack.push((span, false, false));
    }

    /// Records a `return` in the innermost open scope. `has_argument` is `true`
    /// for `return x;` and `false` for a bare `return;`.
    pub(crate) fn record_return(&mut self, has_argument: bool) {
        let Some((_, has_value_return, has_bare_return)) = self.return_kind_stack.last_mut() else {
            return;
        };
        if has_argument {
            *has_value_return = true;
        } else {
            *has_bare_return = true;
        }
    }

    /// Pops the innermost frame and reports the scope when it mixed an explicit
    /// value return with an explicit bare return.
    pub(crate) fn leave_return_scope(&mut self) {
        let Some((span, has_value_return, has_bare_return)) = self.return_kind_stack.pop() else {
            return;
        };
        if has_value_return && has_bare_return {
            self.report(RULE_NAME, "inconsistentReturns", span);
        }
    }
}
