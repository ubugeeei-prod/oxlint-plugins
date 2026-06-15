//! Rule `post-message` (SonarJS key S2819).
//!
//! Clean-room port. Cross-document messaging via `postMessage` is a security
//! hotspot: when the target origin is the wildcard `"*"`, the message is
//! delivered to a window regardless of its origin, so any document loaded at
//! that location can read the data. This implements ONLY the unambiguous,
//! zero-false-positive SENDING subset — a call to `.postMessage(message, "*")`
//! where the second argument is the string literal `"*"`. The receiving side
//! (missing `event.origin` checks) requires dataflow analysis and is out of
//! scope.
//!
//! **Flagged** — a `CallExpression` whose callee is a static member expression
//! named `postMessage`, with at least two arguments, where the second argument
//! is the string literal `"*"`:
//! - `win.postMessage(data, "*")` — wildcard target origin.
//! - `el.postMessage(x, "*")` — the receiver's type is irrelevant; the
//!   `(message, "*")` shape is essentially unique to wildcard-origin usage.
//!
//! **Not flagged**:
//! - `worker.postMessage(data)` — single argument (Worker/MessagePort
//!   signature; no target origin).
//! - `win.postMessage(data, "https://example.com")` — a specific, safe target
//!   origin.
//! - `win.postMessage(data, origin)` — a variable second argument; its value
//!   is not guessed.
//! - `worker.postMessage(data, [transferable])` — array second argument.
//!
//! Behaviour is reproduced from the public RSPEC S2819 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "post-message";

impl Scanner<'_> {
    pub(crate) fn check_post_message(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(m) = expr.callee.get_inner_expression() else {
            return;
        };
        if m.property.name != "postMessage" {
            return;
        }
        if expr.arguments.len() < 2 {
            return;
        }
        let Some(second) = expr.arguments[1].as_expression() else {
            return;
        };
        let Expression::StringLiteral(lit) = second.get_inner_expression() else {
            return;
        };
        if lit.value == "*" {
            self.report(RULE_NAME, "postMessage", expr.span);
        }
    }
}
