//! Rule `web-sql-database` (SonarJS key S2817).
//!
//! Clean-room port. The Web SQL Database API — whose entry point is
//! `openDatabase(...)` — is deprecated and has been removed from the web
//! platform; the specification was abandoned and the API is no longer
//! standardised. Because it exposes a SQL surface to client-side code it is
//! also security-sensitive. This implements the zero-false-positive subset: a
//! call to `openDatabase(...)`, whose name is essentially unique to the WebSQL
//! API.
//!
//! **Flagged** — a `CallExpression` whose callee (after unwrapping
//! parentheses) is either of:
//! - `Expression::Identifier` named `openDatabase` — the global
//!   `openDatabase("db", "1.0", "desc", 1024)`.
//! - `Expression::StaticMemberExpression` whose property is `openDatabase`,
//!   e.g. `window.openDatabase("db")`. The receiver's type is irrelevant;
//!   `openDatabase` is essentially unique to the WebSQL API.
//!
//! **Not flagged**:
//! - `function openDatabase() {}` / `const openDatabase = ...` — a declaration
//!   is not a call.
//! - `window.openDatabase` (no call) — a property access without invocation.
//! - `obj.query()` — an unrelated method name.
//!
//! Behaviour is reproduced from the public RSPEC S2817 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "web-sql-database";

impl Scanner<'_> {
    pub(crate) fn check_web_sql_database(&mut self, call: &CallExpression<'_>) {
        let is_open_database = match call.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident.name == "openDatabase",
            Expression::StaticMemberExpression(member) => member.property.name == "openDatabase",
            _ => false,
        };
        if is_open_database {
            self.report(RULE_NAME, "webSqlDatabase", call.span);
        }
    }
}
