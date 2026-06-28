//! Rule `test-check-exception` (SonarJS key S5958).
//!
//! Clean-room port. When a unit test expects a piece of code to throw, the
//! common pattern is to wrap the code under test in a `try` block and inspect
//! the caught exception in the `catch` block:
//!
//! ```js
//! it('throws', () => {
//!   try {
//!     doSomething();              // expected to throw
//!   } catch (e) {
//!     expect(e.message).to.equal('boom');
//!   }
//! });
//! ```
//!
//! The bug: if `doSomething()` does **not** throw, control never enters the
//! `catch` block, no assertion runs, and the test passes silently — exactly the
//! regression the test was meant to catch. To be correct the test must force a
//! failure when no exception is thrown, e.g. by adding `expect.fail()` /
//! `assert.fail()` / `throw new Error(...)` immediately after the code under
//! test inside the `try` block.
//!
//! ## Narrow, false-positive-free subset
//!
//! Reproducing the full rule needs no type information, so it is implemented,
//! but deliberately restricted to a conservative syntactic shape that is free of
//! false positives. A `try`/`catch` is flagged only when **all** of the
//! following hold:
//!
//! - it is a **top-level** statement in the callback of a known test function
//!   (`it`, `test`, `specify`, plus their `.only` / `.skip` variants) — so we
//!   are certainly inside a test case, not production error handling;
//! - the `catch` clause is present and its body is a flat list of simple
//!   statements that contains at least one assertion call (callee rooted at
//!   `expect` / `assert` / `should`) — strong evidence the developer intends to
//!   verify the exception — and contains **no** `throw` (so re-throwing
//!   error-translation helpers are not flagged);
//! - the `try` block is a flat list of simple statements (no `if`/loops/nested
//!   blocks/nested `try`) that contains **no** fail-forcing construct: no
//!   `throw` statement and no call to `expect` / `assert` / `should` / `fail`
//!   (a bare `fail()` or a `.fail()` member). Any of those would already make
//!   the test fail when nothing is thrown.
//!
//! If a block contains control-flow statements the rule bails out (reports
//! nothing) rather than risk a false positive on a nested fail-forcing call.
//! This under-reports complex cases by design.
//!
//! Reported at the span of the offending `try` statement.
//!
//! Behaviour is reproduced from the public RSPEC description (S5958) and the
//! public Mocha / Chai documentation only; no upstream source, tests, fixtures,
//! or message strings were consulted or copied.

use oxc_ast::ast::{Argument, CallExpression, Expression, Statement, TryStatement};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "test-check-exception";

/// Bare-identifier test functions whose callback may contain a try/catch that
/// checks an expected exception.
const TEST_FNS: [&str; 3] = ["it", "test", "specify"];

/// Returns `true` if `callee` names a known test function (`it`, `test`,
/// `specify`) or one of their `.only` / `.skip` modifiers.
fn is_test_callee(callee: &Expression<'_>) -> bool {
    match callee.get_inner_expression() {
        Expression::Identifier(id) => TEST_FNS.contains(&id.name.as_str()),
        Expression::StaticMemberExpression(member) => {
            let prop = member.property.name.as_str();
            if prop != "only" && prop != "skip" {
                return false;
            }
            matches!(
                member.object.get_inner_expression(),
                Expression::Identifier(object) if TEST_FNS.contains(&object.name.as_str())
            )
        }
        _ => false,
    }
}

/// Walks a member/call chain down to its base identifier and returns its name,
/// e.g. `expect(x).to.equal` → `expect`, `assert.fail` → `assert`.
fn callee_root_name<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
    match expr.get_inner_expression() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(member) => callee_root_name(&member.object),
        Expression::ComputedMemberExpression(member) => callee_root_name(&member.object),
        Expression::CallExpression(call) => callee_root_name(&call.callee),
        _ => None,
    }
}

/// Returns `true` when `expr` is a call whose chain is rooted at a known
/// assertion entry point (`expect` / `assert` / `should`).
fn is_assertion_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr.get_inner_expression() else {
        return false;
    };
    matches!(
        callee_root_name(&call.callee),
        Some("expect") | Some("assert") | Some("should")
    )
}

/// Returns `true` when `expr` is a call that would force the test to fail even
/// if no exception is thrown: an assertion call, a bare `fail()`, or any
/// `*.fail(...)` member call.
fn is_fail_forcing_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr.get_inner_expression() else {
        return false;
    };
    if let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression()
        && member.property.name.as_str() == "fail"
    {
        return true;
    }
    matches!(
        callee_root_name(&call.callee),
        Some("expect") | Some("assert") | Some("should") | Some("fail")
    )
}

/// Returns `true` when the `catch` body is a flat list of simple statements
/// that asserts on the caught exception and does not re-throw it.
fn catch_checks_exception(stmts: &[Statement<'_>]) -> bool {
    let mut found_assertion = false;
    for stmt in stmts {
        match stmt {
            Statement::ExpressionStatement(expr_stmt) => {
                if is_assertion_call(&expr_stmt.expression) {
                    found_assertion = true;
                }
            }
            // Re-throwing means the catch is error translation, not a swallow.
            Statement::ThrowStatement(_) => return false,
            Statement::VariableDeclaration(_)
            | Statement::EmptyStatement(_)
            | Statement::ReturnStatement(_) => {}
            // Any control flow makes shallow analysis unsafe; bail.
            _ => return false,
        }
    }
    found_assertion
}

/// Returns `true` when the `try` block is a non-empty flat list of simple
/// statements with no construct that forces the test to fail when nothing is
/// thrown (no `throw`, no assertion/`fail` call).
fn try_is_unguarded(stmts: &[Statement<'_>]) -> bool {
    if stmts.is_empty() {
        return false;
    }
    for stmt in stmts {
        match stmt {
            Statement::ExpressionStatement(expr_stmt) => {
                if is_fail_forcing_call(&expr_stmt.expression) {
                    return false;
                }
            }
            Statement::ThrowStatement(_) => return false,
            Statement::VariableDeclaration(_)
            | Statement::EmptyStatement(_)
            | Statement::ReturnStatement(_) => {}
            _ => return false,
        }
    }
    true
}

/// Returns the span of `try_stmt` when it is the flagged "unchecked exception"
/// shape: an asserting `catch` paired with an unguarded `try`.
fn try_violation_span(try_stmt: &TryStatement<'_>) -> Option<Span> {
    let handler = try_stmt.handler.as_ref()?;
    let catch_body = &handler.body.body;
    if catch_body.is_empty() {
        return None;
    }
    if !catch_checks_exception(catch_body) {
        return None;
    }
    if !try_is_unguarded(&try_stmt.block.body) {
        return None;
    }
    Some(try_stmt.span)
}

/// Collects the top-level try/catch violation spans inside a test callback
/// argument.
fn callback_violations(argument: &Argument<'_>, out: &mut SmallVec<[Span; 4]>) {
    let statements = match argument {
        Argument::FunctionExpression(func) => match func.body.as_ref() {
            Some(body) => &body.statements,
            None => return,
        },
        Argument::ArrowFunctionExpression(arrow) if !arrow.expression => &arrow.body.statements,
        _ => return,
    };
    for stmt in statements {
        if let Statement::TryStatement(try_stmt) = stmt
            && let Some(span) = try_violation_span(try_stmt)
        {
            out.push(span);
        }
    }
}

impl Scanner<'_> {
    /// Flags a top-level try/catch inside a test callback whose `catch` checks
    /// the exception but whose `try` never forces a failure when nothing throws.
    pub(crate) fn check_test_check_exception(&mut self, call: &CallExpression<'_>) {
        if !is_test_callee(&call.callee) {
            return;
        }
        let mut spans: SmallVec<[Span; 4]> = SmallVec::new();
        for argument in &call.arguments {
            callback_violations(argument, &mut spans);
        }
        for span in spans {
            self.report(RULE_NAME, "checkException", span);
        }
    }
}
