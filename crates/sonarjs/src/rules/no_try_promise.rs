//! Rule `no-try-promise` (SonarJS key S4822).
//!
//! Clean-room port. A `try`/`catch` statement only intercepts *synchronous*
//! exceptions thrown while the `try` body runs (and rejections of promises that
//! are `await`-ed). A promise that is produced but never awaited rejects
//! asynchronously, long after control has left the `try` block, so the `catch`
//! handler will never see that rejection — the error handling is silently
//! broken.
//!
//! ## Narrow form
//!
//! Faithfully reproducing the full SonarJS rule requires the TypeScript type
//! checker to decide whether an arbitrary expression is "thenable". That type
//! information is unavailable in this runtime, so this port enforces only the
//! unambiguous, configuration-independent subset: a *syntactically obvious*
//! promise expression used as a bare statement (or returned) directly inside a
//! `try` block that has a `catch` handler, when the expression is not awaited.
//!
//! An expression is treated as obviously a promise when it is:
//! - `new Promise(...)`;
//! - a `Promise.{resolve,reject,all,race,allSettled,any}(...)` combinator call; or
//! - a `.then(...)` continuation call.
//!
//! ```js
//! try {
//!   doAsync().then(handle);   // Noncompliant: rejection escapes the catch
//! } catch (e) { /* never runs for the rejection */ }
//!
//! try {
//!   await doAsync();          // Compliant: awaited, so catch sees rejections
//! } catch (e) {}
//! ```
//!
//! Restricting to these forms guarantees no false positives regardless of types.
//! Expressions whose promise-ness depends on the declared return type of a
//! called function (`fetchUser()`), and promises stored in variables, are a
//! documented follow-up. A `try` with only a `finally` (no `catch`) is not
//! flagged because there is no rejection handler being misled.
//!
//! Behaviour is reproduced from the public RSPEC description (S4822) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, Statement, TryStatement};
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-try-promise";

const PROMISE_COMBINATORS: [&str; 6] = ["resolve", "reject", "all", "race", "allSettled", "any"];

impl Scanner<'_> {
    pub(crate) fn check_no_try_promise(&mut self, it: &TryStatement<'_>) {
        // Only a `catch` handler can be misled into thinking it guards the
        // promise; a bare `try`/`finally` has nothing to report against.
        if it.handler.is_none() {
            return;
        }
        let mut spans = oxlint_plugins_carton::SmallVec::<[oxc_span::Span; 4]>::new();
        for stmt in &it.block.body {
            let expr = match stmt {
                Statement::ExpressionStatement(es) => &es.expression,
                Statement::ReturnStatement(rs) => match &rs.argument {
                    Some(arg) => arg,
                    None => continue,
                },
                _ => continue,
            };
            if is_unawaited_promise(expr) {
                spans.push(expr.span());
            }
        }
        for span in spans {
            self.report(RULE_NAME, "tryPromise", span);
        }
    }
}

/// Returns `true` when `expr` is a syntactically unambiguous promise value that
/// is not `await`-ed. An `AwaitExpression` is never a match: awaiting funnels
/// any rejection back into the enclosing `try`/`catch`.
fn is_unawaited_promise(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::NewExpression(new_expr) => {
            matches!(&new_expr.callee, Expression::Identifier(id) if id.name.as_str() == "Promise")
        }
        Expression::CallExpression(call) => {
            let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression()
            else {
                return false;
            };
            let prop = member.property.name.as_str();
            if prop == "then" {
                return true;
            }
            if PROMISE_COMBINATORS.contains(&prop)
                && let Expression::Identifier(obj) = member.object.get_inner_expression()
            {
                return obj.name.as_str() == "Promise";
            }
            false
        }
        _ => false,
    }
}
