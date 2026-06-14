//! Rule `standard-input` (SonarJS key S4829).
//!
//! Clean-room port. Reading from the standard input is security-sensitive:
//! the data comes from outside the program and is attacker-controllable, so
//! using it without validation has led to vulnerabilities. In Node.js the
//! standard input is reached through `process.stdin` (the JavaScript analog of
//! the standard-input handles flagged in other languages). This rule is a
//! security hotspot that flags the access so a human can review it.
//!
//! **Flagged** — a `StaticMemberExpression` whose object is the bare
//! `process` identifier and whose property is `stdin`:
//! - `const x = process.stdin;` — direct member access.
//! - `process.stdin.on('data', cb);` — the inner `process.stdin` member access
//!   is visited (the `.on` member is the outer expression).
//! - `process.stdin.read();` — the inner `process.stdin` member access is
//!   visited (the `.read` member is the outer expression).
//!
//! Each of these reports exactly once, on the `process.stdin` member access.
//!
//! **Not flagged**:
//! - `process.stdout;` — different property name.
//! - `foo.stdin;` — object is not the bare `process` identifier.
//! - `stdin;` — bare identifier, not a member expression.
//!
//! Only the bare `process.stdin` member-access form is covered. Other
//! standard-input forms — `require('readline')`, `fs.readSync(0, …)`, or the
//! file descriptor `0` — are out of scope for this syntactic check, as is an
//! indirect `const { stdin } = process;` destructure.
//!
//! Behaviour is reproduced from the public RSPEC S4829 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, StaticMemberExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "standard-input";

impl Scanner<'_> {
    pub(crate) fn check_standard_input(&mut self, member: &StaticMemberExpression<'_>) {
        if member.property.name != "stdin" {
            return;
        }
        let Expression::Identifier(obj) = member.object.get_inner_expression() else {
            return;
        };
        if obj.name == "process" {
            self.report(RULE_NAME, "standardInput", member.span);
        }
    }
}
