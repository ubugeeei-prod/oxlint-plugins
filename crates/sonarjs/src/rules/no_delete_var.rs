//! Rule `no-delete-var` (SonarJS key S3001).
//!
//! Clean-room port. The `delete` operator is designed for removing properties
//! from objects. Applying `delete` to a plain variable identifier (`delete x`)
//! is a no-op in sloppy mode and a `SyntaxError` in strict mode, so it is
//! almost always a programmer mistake.
//!
//! **Flagged**: a `UnaryExpression` with operator `delete` whose argument,
//! after stripping parentheses via `get_inner_expression()`, is a bare
//! `IdentifierReference` (e.g. `delete x`, `delete (y)`).
//!
//! **Not flagged**: `delete obj.prop` or `delete obj[key]` — those are member
//! expressions and are the legitimate use of the operator.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Expression, UnaryExpression};
use oxc_syntax::operator::UnaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-delete-var";

impl Scanner<'_> {
    pub(crate) fn check_no_delete_var(&mut self, expr: &UnaryExpression<'_>) {
        if expr.operator != UnaryOperator::Delete {
            return;
        }
        if !matches!(expr.argument.get_inner_expression(), Expression::Identifier(_)) {
            return;
        }
        self.report(RULE_NAME, "noDeleteVar", expr.span);
    }
}
