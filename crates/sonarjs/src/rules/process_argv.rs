//! Rule `process-argv` (SonarJS key S4823).
//!
//! Clean-room port. Reading command-line arguments via `process.argv` is
//! security-sensitive: those arguments are attacker-controllable input and
//! using them without validation has led to vulnerabilities. This rule is a
//! security hotspot that flags the access so a human can review it.
//!
//! **Flagged** — a `StaticMemberExpression` whose object is the bare
//! `process` identifier and whose property is `argv`:
//! - `const a = process.argv;` — direct member access.
//! - `process.argv[2];` — the inner `process.argv` member access is visited
//!   (the index lives in an outer computed member expression).
//! - `process.argv.slice(2);` — the inner `process.argv` member access is
//!   visited (the `.slice` member is the outer expression).
//!
//! Each of these reports exactly once, on the `process.argv` member access.
//!
//! **Not flagged**:
//! - `process.env.PATH;` — different property name.
//! - `foo.argv;` — object is not the bare `process` identifier.
//! - `argv;` — bare identifier, not a member expression.
//!
//! Only the bare `process.argv` member-access form is covered. Indirect forms
//! such as `require('process').argv` or a destructured `const { argv } =
//! process;` are out of scope for this syntactic check.
//!
//! Behaviour is reproduced from the public RSPEC S4823 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, StaticMemberExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "process-argv";

impl Scanner<'_> {
    pub(crate) fn check_process_argv(&mut self, member: &StaticMemberExpression<'_>) {
        if member.property.name != "argv" {
            return;
        }
        let Expression::Identifier(obj) = member.object.get_inner_expression() else {
            return;
        };
        if obj.name == "process" {
            self.report(RULE_NAME, "processArgv", member.span);
        }
    }
}
