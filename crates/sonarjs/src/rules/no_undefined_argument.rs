//! Rule `no-undefined-argument` (SonarJS key S4623).
//!
//! Clean-room port. Passing `undefined` explicitly as the **last** argument to
//! a function or constructor call is redundant: omitting it produces identical
//! runtime behaviour because the parameter receives `undefined` either way.
//!
//! ## Flagged
//!
//! ```js
//! foo(1, undefined);   // Noncompliant — trailing `undefined`
//! foo(undefined);      // Noncompliant — sole argument is `undefined`
//! new Foo(undefined);  // Noncompliant — same rule for constructor calls
//! ```
//!
//! **Not flagged**:
//! - `foo(undefined, 1)` — `undefined` is not the last argument.
//! - `foo()` — no arguments at all.
//! - `foo(...undefined)` — spread argument; not a bare identifier.
//! - `foo(1, 2)` — no `undefined` argument.
//!
//! Behaviour is reproduced from the public RSPEC description (S4623) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Argument, CallExpression, NewExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-undefined-argument";

impl Scanner<'_> {
    pub(crate) fn check_no_undefined_argument_call(&mut self, call: &CallExpression<'_>) {
        self.check_no_undefined_argument_last(&call.arguments);
    }

    pub(crate) fn check_no_undefined_argument_new(&mut self, new_expr: &NewExpression<'_>) {
        self.check_no_undefined_argument_last(&new_expr.arguments);
    }

    fn check_no_undefined_argument_last(&mut self, arguments: &[Argument<'_>]) {
        let Some(last) = arguments.last() else {
            return;
        };
        let Argument::Identifier(ident) = last else {
            return;
        };
        if ident.name == "undefined" {
            self.report(RULE_NAME, "removeUndefined", ident.span);
        }
    }
}
