//! Rule `no-redundant-jump` (SonarJS key S3626).
//!
//! Clean-room port. A jump statement that does not change control flow is
//! redundant. This rule covers two cases:
//!
//! 1. **`continue;`** (with no label) as the last statement in a loop body
//!    block — the loop iteration would end at that point anyway.
//! 2. **`return;`** (with no argument) as the last statement in a function
//!    body — the function would return there anyway.
//!
//! Labeled `continue` statements are **not** flagged (they change which loop
//! is continued). `return` statements with a value are **not** flagged (they
//! carry a result). Non-block loop bodies (e.g. `while (x) foo();`) are also
//! not flagged.
//!
//! ## Flagged
//!
//! ```js
//! for (;;) { foo(); continue; }
//! while (x) { foo(); continue; }
//! do { foo(); continue; } while (x);
//! for (const a of b) { foo(); continue; }
//! for (k in o) { foo(); continue; }
//! function f() { foo(); return; }
//! const g = () => { foo(); return; };
//! ```
//!
//! ## Not flagged
//!
//! ```js
//! // continue is not the last statement
//! for (;;) { if (x) continue; foo(); }
//! // labeled continue — changes which loop is continued
//! outer: for (;;) { foo(); continue outer; }
//! // return with a value
//! function f() { foo(); return x; }
//! // no trailing return
//! function f() { foo(); }
//! // non-block loop body
//! while (x) foo();
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{FunctionBody, Statement};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-redundant-jump";

/// Returns the span of a redundant trailing `continue;` in a loop body, or
/// `None` if the body has no such statement.
///
/// Only block bodies are checked. Non-block bodies cannot end with a bare
/// `continue;` that is also the *only* thing being done (they are a single
/// statement anyway), so they are intentionally excluded.
fn trailing_redundant_continue(body: &Statement) -> Option<Span> {
    let Statement::BlockStatement(block) = body else {
        return None;
    };
    let Statement::ContinueStatement(cont) = block.body.last()? else {
        return None;
    };
    if cont.label.is_some() {
        return None;
    }
    Some(cont.span)
}

impl Scanner<'_> {
    /// Check a loop body for a redundant trailing `continue;`.
    pub(crate) fn check_redundant_continue(&mut self, loop_body: &Statement<'_>) {
        let Some(span) = trailing_redundant_continue(loop_body) else {
            return;
        };
        self.report(RULE_NAME, "redundantJump", span);
    }

    /// Check a function body for a redundant trailing `return;`.
    pub(crate) fn check_redundant_return(&mut self, body: &FunctionBody<'_>) {
        let Some(Statement::ReturnStatement(ret)) = body.statements.last() else {
            return;
        };
        if ret.argument.is_some() {
            return;
        }
        self.report(RULE_NAME, "redundantJump", ret.span);
    }
}
