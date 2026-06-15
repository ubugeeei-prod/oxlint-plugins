//! Rule `production-debug` (SonarJS key S2228).
//!
//! Clean-room port. Shipping code with debugging features still active is a
//! security and quality hotspot: a `debugger` statement halts execution under a
//! debugger and has no legitimate purpose in delivered code. This port
//! implements the zero-false-positive syntactic subset and flags every
//! `debugger` statement, which has exactly one meaning and cannot be confused
//! with anything else.
//!
//! **Flagged** — the JavaScript `debugger` statement:
//! - `debugger;` at the top level.
//! - `function f() { debugger; }` — inside a function body.
//! - `if (x) { debugger; }` — inside any nested block.
//!
//! **Not flagged** (intentionally narrow, to stay zero-false-positive):
//! - `console.log(...)`, `console.debug(...)`, and any other `console.*` call —
//!   these are frequently legitimate logging and are never flagged.
//! - `alert(...)`, `confirm(...)`, `prompt(...)` — also frequently legitimate
//!   and never flagged.
//! - Ordinary statements such as `return 1;` — only the `debugger` statement is
//!   reported.
//!
//! Behaviour is reproduced from the public RSPEC S2228 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::DebuggerStatement;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "production-debug";

impl Scanner<'_> {
    pub(crate) fn check_production_debug(&mut self, stmt: &DebuggerStatement) {
        self.report(RULE_NAME, "productionDebug", stmt.span);
    }
}
