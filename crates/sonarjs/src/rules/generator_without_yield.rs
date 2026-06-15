//! Rule `generator-without-yield` (SonarJS key S3531).
//!
//! Clean-room port. Reports a generator function (`function*`) whose own body
//! contains no `yield` expression directly inside it, because such a generator
//! behaves like a plain function that returns an iterator yielding nothing — almost
//! always a mistake.
//!
//! ## Stack-based generator tracking
//!
//! A `yield` expression is syntactically valid **only** inside a generator function,
//! and it always binds to the **nearest enclosing generator** — it can never cross
//! a non-generator function boundary (that would be a SyntaxError). Therefore,
//! maintaining a stack of boolean frames (one per open generator) and marking the
//! **top** frame when a `yield` is encountered correctly attributes each yield to
//! the right (innermost) generator.
//!
//! ### Nested-generator example
//!
//! ```js
//! function* outer() {
//!     function* inner() { yield 1; }   // inner has yield → only outer flagged
//! }
//! ```
//!
//! Walk order:
//! 1. Enter `outer` → push `false` (stack: `[false]`)
//! 2. Enter `inner` → push `false` (stack: `[false, false]`)
//! 3. Visit `yield 1` → mark top: `[false, true]`
//! 4. Leave `inner` → pop `true` → inner had yield → **no report for inner**
//! 5. Leave `outer` → pop `false` → outer had no yield → **report outer**
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::Function;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "generator-without-yield";

impl<'a> Scanner<'a> {
    /// Called before walking a function node. Pushes a tracking frame onto the
    /// generator stack if the function is a generator with a body. Returns
    /// `true` when a frame was pushed (i.e., the caller must call
    /// [`leave_generator`](Scanner::leave_generator) with `true`).
    pub(crate) fn enter_generator(&mut self, func: &Function<'a>) -> bool {
        let track = func.generator && func.body.is_some();
        if track {
            self.generator_yield_stack.push(false);
        }
        track
    }

    /// Called after walking a function node. Pops the frame and reports the
    /// generator if it contained no `yield`.
    pub(crate) fn leave_generator(&mut self, func: &Function<'a>, track: bool) {
        if !track {
            return;
        }
        let had_yield = self.generator_yield_stack.pop().unwrap_or(true);
        if !had_yield {
            self.report(RULE_NAME, "generatorWithoutYield", func.span);
        }
    }

    /// Marks the innermost open generator frame as having seen a `yield`.
    pub(crate) fn mark_generator_yield(&mut self) {
        if let Some(top) = self.generator_yield_stack.last_mut() {
            *top = true;
        }
    }
}
