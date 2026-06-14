//! Rule `pseudo-random` (SonarJS key S2245).
//!
//! Clean-room port. Flags the use of `Math.random()`, a pseudorandom number
//! generator that is NOT cryptographically secure and must not be used in
//! security-sensitive contexts such as session-ID generation, password
//! creation, or cryptographic operations.
//!
//! **Flagged** — a `CallExpression` whose callee is the static member
//! expression `Math.random`:
//! - `Math.random()` — direct call to the PRNG.
//! - `const x = Math.random();` — assigned but still a call.
//!
//! **Not flagged**:
//! - `Math.floor(x)` — different property name.
//! - `foo.random()` — object is not the bare `Math` identifier.
//! - `random()` — bare identifier, not a member expression.
//! - `const f = Math.random;` — bare reference without a call; only the
//!   call form is reported.
//!
//! Behaviour is reproduced from the public RSPEC S2245 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "pseudo-random";

impl Scanner<'_> {
    pub(crate) fn check_pseudo_random(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(m) = expr.callee.get_inner_expression() else {
            return;
        };
        if m.property.name != "random" {
            return;
        }
        let Expression::Identifier(obj) = m.object.get_inner_expression() else {
            return;
        };
        if obj.name == "Math" {
            self.report(RULE_NAME, "pseudoRandom", expr.span);
        }
    }
}
