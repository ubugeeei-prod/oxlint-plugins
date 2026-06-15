//! Rule `no-incomplete-assertions` (SonarJS key S2970).
//!
//! Clean-room port. In Chai's BDD style an `expect(value)` call returns an
//! Assertion object; you then chain a terminal assertion property or method
//! (e.g. `.to.be.true`, `.to.equal(42)`) to actually check something. When
//! the chain is never finished — because the statement stops at the bare call
//! or at one of Chai's non-asserting language-chain getters — the test always
//! silently passes regardless of the value under test.
//!
//! **Flagged** (expression statements only):
//! - `expect(x);` — bare call, no chain at all.
//! - `expect(x).to;` — ends on the language-chain getter `.to`.
//! - `expect(x).to.be;` — ends on the language-chain getter `.be`.
//! - `expect(x).not;` — `.not` is a non-asserting flag getter.
//! - `expect(x).to.have.deep;` — any chain that ends on a language-chain property.
//!
//! **Not flagged**:
//! - `expect(x).to.be.true;` — `.true` is a terminal assertion getter.
//! - `expect(x).to.equal(42);` — `.equal(42)` is a method call (terminal).
//! - `expect(x).to.be.an('array');` — method call is terminal.
//! - `foo(x);` — callee is not `expect`.
//! - `chai.expect(x);` — callee is a member expression, not a bare identifier.
//! - `const a = expect(x).to;` — not a statement, the value is used.
//!
//! ## Language chains
//! Chai's public BDD API documentation lists the following as chainability-only
//! getters with no assertion behaviour: `to`, `be`, `been`, `is`, `that`,
//! `which`, `and`, `has`, `have`, `with`, `at`, `of`, `same`, `but`, `does`,
//! `still`. The negation flag `not` is also non-terminal by itself.
//!
//! Behaviour is derived from Chai's public API documentation and the SonarJS
//! S2970 rule description only; no upstream source, tests, fixtures, or message
//! strings were consulted or copied.

use oxc_ast::ast::{Expression, ExpressionStatement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-incomplete-assertions";

/// Returns `true` for Chai BDD property names that have no assertion semantics
/// and are documented as language-chain getters. An expression statement that
/// terminates on one of these leaves the assertion incomplete.
fn is_language_chain(name: &str) -> bool {
    matches!(
        name,
        "to" | "be"
            | "been"
            | "is"
            | "that"
            | "which"
            | "and"
            | "has"
            | "have"
            | "with"
            | "at"
            | "of"
            | "same"
            | "but"
            | "does"
            | "still"
            | "not"
    )
}

/// Walks a chain of static-member-expression nodes and returns the innermost
/// expression (the root of the chain), stopping when a non-member node is
/// reached. Parenthesised expressions are unwrapped transparently.
fn chain_root<'a>(mut expr: &'a Expression<'a>) -> &'a Expression<'a> {
    loop {
        match expr {
            Expression::StaticMemberExpression(member) => {
                expr = &member.object;
            }
            Expression::ParenthesizedExpression(paren) => {
                expr = &paren.expression;
            }
            _ => return expr,
        }
    }
}

/// Returns `true` when `expr` is a direct call to the bare `expect` identifier
/// (not a namespaced call such as `chai.expect(...)`).
fn is_bare_expect_call(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::CallExpression(call) => matches!(
            call.callee.get_inner_expression(),
            Expression::Identifier(id) if id.name == "expect"
        ),
        Expression::ParenthesizedExpression(paren) => is_bare_expect_call(&paren.expression),
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_incomplete_assertions(&mut self, it: &ExpressionStatement<'_>) {
        let expr = it.expression.get_inner_expression();
        match expr {
            // `expect(x);` — bare call with no chained assertion.
            Expression::CallExpression(call)
                if matches!(
                    call.callee.get_inner_expression(),
                    Expression::Identifier(id) if id.name == "expect"
                ) =>
            {
                self.report(RULE_NAME, "incompleteAssertion", it.span);
            }
            // `expect(x).to;` / `expect(x).to.be;` / … — chain ends on a
            // non-asserting language-chain getter whose root is `expect(…)`.
            Expression::StaticMemberExpression(member)
                if is_language_chain(member.property.name.as_str())
                    && is_bare_expect_call(chain_root(expr)) =>
            {
                self.report(RULE_NAME, "incompleteAssertion", it.span);
            }
            _ => {}
        }
    }
}
