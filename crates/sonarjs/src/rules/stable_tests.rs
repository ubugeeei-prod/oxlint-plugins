//! Rule `stable-tests` (SonarJS key S5973).
//!
//! Clean-room port. Flaky tests pass sometimes and fail other times without any
//! code change. Some runners let you paper over the flakiness by automatically
//! re-running a failing test a few times and reporting success if any attempt
//! passes. That hides the instability instead of fixing it, so a genuine
//! regression can slip through to production. The RSPEC for S5973 raises an
//! issue when a test re-run is requested with a positive count via:
//!   * `jest.retryTimes(n)` — Jest's global retry configuration.
//!   * `this.retries(n)` — Mocha's per-test/suite retry configuration.
//!
//! This implements ONLY the unambiguous, zero-false-positive subset: a
//! `CallExpression` whose callee is a static member access of the distinctive
//! shape `jest.retryTimes(<positive numeric literal>)` or
//! `this.retries(<positive numeric literal>)`, where the receiver is the bare
//! `jest` identifier or the `this` keyword respectively, and the first argument
//! is a numeric literal strictly greater than `0`.
//!
//! **Flagged**:
//! - `jest.retryTimes(3)` — re-runs every failing test up to three times.
//! - `jest.retryTimes(1, { logErrorsBeforeRetry: true })` — first arg is the count.
//! - `this.retries(2)` — Mocha per-test retry.
//!
//! **Not flagged**:
//! - `jest.retryTimes(0)` / `this.retries(0)` — explicitly disables retries.
//! - `jest.retryTimes(n)` / `this.retries(n)` — non-literal count; not guessed.
//! - `jest.retryTimes()` — no count argument supplied.
//! - `foo.retryTimes(3)` / `obj.retries(3)` — receiver is not `jest` / `this`.
//!
//! The Mocha `this.retries(n)` form is reported wherever the distinctive
//! `this.retries(<positive literal>)` shape appears; under-reporting (e.g. for a
//! count stored in a variable) is preferred over guessing.
//!
//! Behaviour is reproduced from the public RSPEC S5973 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "stable-tests";

impl Scanner<'_> {
    pub(crate) fn check_stable_tests(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };

        // Match `jest.retryTimes(...)` (bare `jest` receiver) or
        // `this.retries(...)` (the `this` keyword receiver); anything else is
        // not the distinctive retry-configuration shape.
        let is_retry_call = match member.property.name.as_str() {
            "retryTimes" => matches!(
                member.object.get_inner_expression(),
                Expression::Identifier(id) if id.name == "jest"
            ),
            "retries" => matches!(
                member.object.get_inner_expression(),
                Expression::ThisExpression(_)
            ),
            _ => false,
        };
        if !is_retry_call {
            return;
        }

        // The retry count is the first argument; it must be a numeric literal so
        // that we only flag values we can prove are positive.
        let Some(first) = expr.arguments.first().and_then(|arg| arg.as_expression()) else {
            return;
        };
        let Expression::NumericLiteral(lit) = first.get_inner_expression() else {
            return;
        };
        if lit.value > 0.0 {
            self.report(RULE_NAME, "stableTests", expr.span);
        }
    }
}
