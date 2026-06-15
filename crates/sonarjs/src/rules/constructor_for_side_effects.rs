//! Rule `constructor-for-side-effects` (SonarJS key S1848).
//!
//! Clean-room port. Creating an object with `new` and immediately discarding
//! the result (using the expression as a bare statement) is suspicious: either
//! the constructor has side effects — which should instead live in a named
//! function or method — or the result was meant to be assigned but the
//! assignment was forgotten.
//!
//! **Flagged**: an `ExpressionStatement` whose `expression`, after stripping
//! parentheses via `get_inner_expression()`, is a `NewExpression`
//! (e.g. `new Foo();`, `new Bar`).
//!
//! **Not flagged**:
//! - `const x = new Foo();` — the result is captured in a variable.
//! - `new Foo().bar();` — the AST node is a `CallExpression` statement;
//!   the `new` result is used as a receiver.
//! - `foo();` — a plain call expression, not `new`.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Expression, ExpressionStatement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "constructor-for-side-effects";

impl Scanner<'_> {
    pub(crate) fn check_constructor_for_side_effects(&mut self, stmt: &ExpressionStatement<'_>) {
        if !matches!(
            stmt.expression.get_inner_expression(),
            Expression::NewExpression(_)
        ) {
            return;
        }
        self.report(RULE_NAME, "constructorForSideEffects", stmt.span);
    }
}
