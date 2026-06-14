//! Rule `no-unthrown-error` (SonarJS key S3984).
//!
//! Clean-room port. Creating an `Error` (or an Error subtype) with `new` and
//! immediately discarding the result as a bare statement is almost certainly a
//! bug: the developer most likely meant to `throw` it.
//!
//! **Heuristic**: the callee identifier name ends with `"Error"`. This covers
//! the built-in types (`Error`, `TypeError`, `RangeError`, `SyntaxError`,
//! `ReferenceError`, `EvalError`, `URIError`, `AggregateError`) and any
//! user-defined class whose name follows the same convention (`FooError`,
//! `MyError`, etc.). Names that do not end with `"Error"` (e.g. `new Foo()`)
//! are left to other rules.
//!
//! **Flagged**: an `ExpressionStatement` whose `expression`, after stripping
//! parentheses via `get_inner_expression()`, is a `NewExpression` whose
//! `callee`, after the same stripping, is an `Identifier` whose name ends
//! with `"Error"` (case-sensitive; `new error()` is NOT flagged).
//!
//! **Not flagged**:
//! - `throw new Error('boom');` — a `ThrowStatement`, not an
//!   `ExpressionStatement`; `visit_expression_statement` never sees it.
//! - `const e = new Error();` — captured in a variable; the statement node is
//!   a `VariableDeclaration`, not an `ExpressionStatement`.
//! - `new Foo();` — the callee name `"Foo"` does not end with `"Error"`.
//! - `foo(new Error());` — the statement's expression is a `CallExpression`
//!   (not a `NewExpression`); the check returns early before inspecting the
//!   callee.
//! - `new error();` — lower-case; does not match the suffix `"Error"`.
//!
//! Behaviour is reproduced from the public RSPEC description (S3984) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, ExpressionStatement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-unthrown-error";

impl Scanner<'_> {
    pub(crate) fn check_no_unthrown_error(&mut self, stmt: &ExpressionStatement<'_>) {
        let Expression::NewExpression(new_expr) = stmt.expression.get_inner_expression() else {
            return;
        };
        let Expression::Identifier(callee) = new_expr.callee.get_inner_expression() else {
            return;
        };
        if !callee.name.as_str().ends_with("Error") {
            return;
        }
        self.report(RULE_NAME, "unthrownError", stmt.span);
    }
}
