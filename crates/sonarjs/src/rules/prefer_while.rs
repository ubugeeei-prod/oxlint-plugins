//! Rule `prefer-while` (SonarJS key S1264).
//!
//! Clean-room port. Reports a `for` statement that has no initializer and no
//! update clause, because such a loop is semantically equivalent to a `while`
//! loop and should use `while` for clarity.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `for (; i < 10;) { i++; }` — no init, no update
//! - `for (;;) {}` — no init, no test, no update (equivalent to `while (true)`)
//!
//! ## Not flagged
//! - `for (let i = 0; i < 10;) {}` — has init
//! - `for (; i < 10; i++) {}` — has update
//! - `for (let i = 0; i < 10; i++) {}` — has both init and update
//! - `while (x) {}` — not a for statement

use oxc_ast::ast::ForStatement;
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-while";

impl Scanner<'_> {
    pub(crate) fn check_prefer_while(&mut self, stmt: &ForStatement<'_>) {
        if stmt.init.is_some() || stmt.update.is_some() {
            return;
        }
        let start = stmt.span.start;
        self.report(RULE_NAME, "preferWhile", Span::new(start, start + 3));
    }
}
