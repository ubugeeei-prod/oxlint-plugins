//! Rule `for-in` (SonarJS key S1535).
//!
//! Clean-room port. Reports a `for...in` loop whose body does not consist of a
//! single `if` statement. A `for...in` loop iterates over both own and inherited
//! enumerable properties, so conventional practice is to guard the body with an
//! `if` statement (e.g. `if (Object.prototype.hasOwnProperty.call(obj, key))`)
//! to filter out inherited properties. This rule enforces that structural shape
//! only — it checks whether the body is a single `if` statement, but does NOT
//! inspect the `if` condition itself. Any `if` guard satisfies the rule.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `for (k in o) { doStuff(k); }` — body is not a single `if`
//! - `for (k in o) doStuff(k);` — single non-`if` statement
//! - `for (k in o) {}` — empty block contains no `if`
//! - `for (k in o) { if (a) {} doStuff(); }` — block has two statements
//!
//! ## Not flagged
//! - `for (k in o) { if (o.hasOwnProperty(k)) { doStuff(k); } }` — single `if` in block
//! - `for (k in o) if (cond) doStuff();` — body is directly an `if` statement

use oxc_ast::ast::{ForInStatement, Statement};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "for-in";

/// Returns `true` when `body` is either directly an `IfStatement` or a
/// `BlockStatement` containing exactly one `IfStatement`.
fn is_single_if_body(body: &Statement<'_>) -> bool {
    match body {
        Statement::IfStatement(_) => true,
        Statement::BlockStatement(block) => {
            block.body.len() == 1 && matches!(block.body.first(), Some(Statement::IfStatement(_)))
        }
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_for_in(&mut self, stmt: &ForInStatement<'_>) {
        if is_single_if_body(&stmt.body) {
            return;
        }
        let start = stmt.span.start;
        self.report(RULE_NAME, "forIn", Span::new(start, start + 3));
    }
}
