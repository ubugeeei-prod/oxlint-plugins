//! Rule `disabled-timeout` (SonarJS key S6080).
//!
//! Clean-room port. In Mocha, `this.timeout(N)` configures the per-test/suite
//! timeout in milliseconds. The value is ultimately passed to `setTimeout`,
//! whose delay is stored in a signed 32-bit integer; a value greater than
//! `2147483647` (the maximum signed 32-bit integer) overflows that range and is
//! silently coerced to `0`, which Mocha interprets as "no timeout". A developer
//! who writes a very large number intending a long-but-finite timeout instead
//! disables the timeout entirely, which is almost never what they meant.
//!
//! This implements ONLY the unambiguous, zero-false-positive subset: a
//! `CallExpression` whose callee is a static member expression with a
//! `ThisExpression` object and the property name `timeout`, called with exactly
//! one argument that is a numeric literal whose value exceeds `2147483647`.
//! The `this.timeout(<huge number>)` shape is distinctive to Mocha.
//!
//! **Flagged**:
//! - `this.timeout(2147483648)` — one past the 32-bit maximum.
//! - `this.timeout(9999999999)` — far past the maximum.
//!
//! **Not flagged**:
//! - `this.timeout(0)` — intentionally disables the timeout; the clear,
//!   recommended idiom.
//! - `this.timeout(5000)` — a value within the valid 32-bit range.
//! - `foo.timeout(2147483648)` — the receiver is not `this`.
//! - `this.timeout(x)` — a non-literal argument; its value is not guessed.
//!
//! Behaviour is reproduced from the public RSPEC S6080 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "disabled-timeout";

/// Maximum signed 32-bit integer; `setTimeout` delays above this overflow and
/// are treated as `0` (no timeout).
const MAX_32_BIT_DELAY: f64 = 2_147_483_647.0;

impl Scanner<'_> {
    pub(crate) fn check_disabled_timeout(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        if member.property.name != "timeout" {
            return;
        }
        if !matches!(
            member.object.get_inner_expression(),
            Expression::ThisExpression(_)
        ) {
            return;
        }
        if expr.arguments.len() != 1 {
            return;
        }
        let Some(arg) = expr.arguments[0].as_expression() else {
            return;
        };
        let Expression::NumericLiteral(lit) = arg.get_inner_expression() else {
            return;
        };
        if lit.value > MAX_32_BIT_DELAY {
            self.report(RULE_NAME, "disabledTimeout", expr.span);
        }
    }
}
