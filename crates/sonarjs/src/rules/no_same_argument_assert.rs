//! Rule `no-same-argument-assert` (SonarJS key S5863).
//!
//! Clean-room port. A Chai-style `assert` call that is given the SAME
//! expression as both its actual and its expected argument is a bug: such an
//! assertion compares a value to itself and is therefore trivially true (or, for
//! ordering checks such as `isAbove`, trivially false) regardless of the code
//! under test, so it verifies nothing.
//!
//! **Flagged** — a `CallExpression` whose callee is the static member expression
//! `assert.<method>` (the object being the bare identifier `assert`, Chai's
//! `assert` interface) that has at least two arguments where the first two are
//! the same source text:
//! - `assert.equal(x, x);`
//! - `assert.strictEqual(foo.bar, foo.bar);`
//! - `assert.isAbove(value, value);`
//!
//! **Not flagged**:
//! - `assert.equal(x, y);` — the first two arguments differ.
//! - `assert.ok(x);` — only one argument, so there is no pair to compare.
//! - `foo(x, x);` — not an `assert` member call; a generic call with repeated
//!   arguments is not necessarily a mistake, so only assertion calls are checked
//!   (conservative, to avoid false positives).
//! - `chai.assert.equal(x, x);` — the object of the callee is itself a member
//!   expression, not the bare identifier `assert`; out of scope for this
//!   syntactic check.
//! - `assert.equal(...xs, ...xs);` — a spread element is not a plain expression,
//!   so the pair is skipped.
//!
//! ## Argument identity
//! The first two arguments are "the same" iff
//! `self.text(arg0.span()) == self.text(arg1.span())`. This is a **syntactic**
//! (source-text) comparison; it does not account for semantic equivalence or
//! aliasing.
//!
//! Behaviour is reproduced from the public RSPEC S5863 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-same-argument-assert";

impl Scanner<'_> {
    pub(crate) fn check_no_same_argument_assert(&mut self, call: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        if object.name != "assert" {
            return;
        }
        let [first, second, ..] = call.arguments.as_slice() else {
            return;
        };
        if first.is_spread() || second.is_spread() {
            return;
        }
        if self.text(first.span()) == self.text(second.span()) {
            self.report(RULE_NAME, "sameArgumentAssert", call.span);
        }
    }
}
