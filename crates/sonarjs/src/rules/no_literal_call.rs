//! Rule `no-literal-call` (SonarJS key S6958).
//!
//! Clean-room port. A literal can never be a function, so attempting to call
//! one (or to use one as a tagged-template tag) always throws a `TypeError` at
//! runtime — it is almost certainly an unintentional mistake.
//!
//! **Flagged** — the callee of a call expression, or the tag of a
//! tagged-template expression, is a literal:
//! - `true()` — a boolean literal invoked as a function.
//! - `42()` / `"foo"()` / `null()` / `1n()` / `/re/()` — other literals.
//! - `` `foo`() `` — a template literal invoked as a function.
//! - `` true`text` `` — a literal used as a tagged-template tag.
//! - `("foo")()` — parentheses around the literal do not change anything.
//!
//! **Not flagged**:
//! - `foo()`, `obj.method()`, `(() => {})()` — callable callees.
//! - `({})()` / `[]()` — object and array expressions are not literals.
//! - `` foo`text` `` — a callable tag.
//!
//! Behaviour is reproduced from the public RSPEC description (S6958,
//! "Literals should not be used as functions") only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{CallExpression, Expression, TaggedTemplateExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-literal-call";

/// Returns `true` when `expr` (after unwrapping parentheses) is a literal that
/// can never be a function: a string, number, boolean, null, bigint, regex, or
/// template literal.
fn is_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr.get_inner_expression(),
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
    )
}

impl Scanner<'_> {
    pub(crate) fn check_no_literal_call(&mut self, expr: &CallExpression<'_>) {
        if !is_literal(&expr.callee) {
            return;
        }
        self.report(RULE_NAME, "noLiteralCall", expr.span);
    }

    pub(crate) fn check_no_literal_tagged_template(&mut self, expr: &TaggedTemplateExpression<'_>) {
        if !is_literal(&expr.tag) {
            return;
        }
        self.report(RULE_NAME, "noLiteralCall", expr.span);
    }
}
