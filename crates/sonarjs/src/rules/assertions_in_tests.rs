//! Rule `assertions-in-tests` (SonarJS key S2699).
//!
//! Clean-room port. A unit test that runs code but never checks any outcome
//! cannot fail for the right reasons: it passes as long as nothing throws,
//! giving a false sense of safety. Every test case should contain at least one
//! assertion.
//!
//! ## Narrow, false-positive-free form
//!
//! Detecting "this test asserts something" in general would require knowing the
//! full surface of every assertion library (Chai, Jest/Vitest `expect`, Node
//! `assert`, Sinon, Supertest, Cypress, custom helpers, ...) and, worse,
//! resolving whether a called helper itself asserts. The runtime has no type
//! information and no cross-function dataflow, so a list-based "is this an
//! assertion call" check would inevitably produce false positives on tests that
//! delegate their assertions to a helper function.
//!
//! This port therefore reports ONLY the unambiguous case: a test-case callback
//! that contains **no invocation of any kind** — no call expression, no `new`
//! expression, and no tagged-template — anywhere in its body (including nested
//! functions). Because every real assertion ultimately involves at least one
//! invocation (`expect(x)`, `assert(...)`, `.toBe(...)`, `x.should.equal(...)`,
//! `cy.get(...)`, a tagged-template matcher, ...), a body with zero invocations
//! provably contains zero assertions. This guarantees no false positives at the
//! cost of under-reporting tests that invoke non-asserting code (those are a
//! documented follow-up that would need an assertion-API allow-list).
//!
//! ## Scope
//!
//! - Only bare test-case callees `it`, `test`, and `specify` are considered.
//!   Suite functions (`describe`, `context`, `suite`) are not test cases and are
//!   never flagged directly; their nested `it`/`test` calls are each checked.
//! - `it.skip`/`it.only`/`it.each` (member-expression callees) are skipped: a
//!   focused or skipped test is reported by other rules, not this one.
//! - A test with no function argument (`it("pending")`) is a pending/todo test
//!   and is never flagged.
//!
//! **Flagged**:
//! ```js
//! it("does nothing", () => {});
//! test("computes without checking", () => { const r = 1 + 2; });
//! ```
//!
//! **Not flagged**:
//! ```js
//! it("adds", () => { expect(1 + 1).toBe(2); });
//! it("delegates", () => { assertUserIsValid(user); }); // helper invocation present
//! it("pending");                                       // no callback
//! it.skip("later", () => {});                          // member callee
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description (S2699) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{
    CallExpression, Expression, NewExpression, Statement, TaggedTemplateExpression,
};
use oxc_ast_visit::{Visit, walk};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "assertions-in-tests";

const TEST_CASE_NAMES: [&str; 3] = ["it", "test", "specify"];

/// Walks a test-case callback body and records whether it contains any
/// invocation (call, `new`, or tagged-template). The presence of even one
/// invocation means the test *might* assert, so it is left unflagged.
struct InvocationFinder {
    found: bool,
}

impl<'a> Visit<'a> for InvocationFinder {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.found = true;
        walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, new_expr: &NewExpression<'a>) {
        self.found = true;
        walk::walk_new_expression(self, new_expr);
    }

    fn visit_tagged_template_expression(&mut self, tagged: &TaggedTemplateExpression<'a>) {
        self.found = true;
        walk::walk_tagged_template_expression(self, tagged);
    }
}

/// Returns the statements of a test-case callback argument, or `None` when the
/// expression is not an arrow/function expression with a usable body.
fn callback_statements<'a, 'b>(expr: &'b Expression<'a>) -> Option<&'b [Statement<'a>]> {
    match expr {
        Expression::ArrowFunctionExpression(arrow) => Some(&arrow.body.statements[..]),
        Expression::FunctionExpression(func) => func.body.as_ref().map(|body| &body.statements[..]),
        _ => None,
    }
}

impl<'a> Scanner<'a> {
    pub(crate) fn check_assertions_in_tests(&mut self, it: &CallExpression<'a>) {
        // Callee must be a bare `it`/`test`/`specify` identifier; member-expression
        // callees (`it.skip`, `it.only`) and suite functions are out of scope.
        let Expression::Identifier(callee) = it.callee.get_inner_expression() else {
            return;
        };
        if !TEST_CASE_NAMES.contains(&callee.name.as_str()) {
            return;
        }

        // Find the test body: the first argument that is a function/arrow with a
        // block body. A test with no callback is a pending test and is skipped.
        let mut statements: Option<&[Statement<'a>]> = None;
        for arg in &it.arguments {
            if let Some(expr) = arg.as_expression()
                && let Some(stmts) = callback_statements(expr.get_inner_expression())
            {
                statements = Some(stmts);
                break;
            }
        }
        let Some(statements) = statements else {
            return;
        };

        let mut finder = InvocationFinder { found: false };
        for stmt in statements {
            finder.visit_statement(stmt);
            if finder.found {
                return;
            }
        }

        // Zero invocations in the whole body => provably zero assertions.
        self.report(RULE_NAME, "addAssertion", it.span);
    }
}
