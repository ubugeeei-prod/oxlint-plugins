//! Rule `no-code-after-done` (SonarJS key S6079).
//!
//! Clean-room port. In a Mocha-style asynchronous test or hook, the test
//! function receives a `done` callback as a parameter; calling `done()` signals
//! the framework that the (asynchronous) test has finished. Any statement that
//! appears **after** the `done()` call still executes, but it runs after the
//! test has already been reported as complete, so its effect is misleading and
//! almost always a bug (a forgotten `return`, a misplaced assertion, etc.).
//!
//! This rule flags the first statement that follows a top-level `done()` call
//! in the body of a Mocha test/hook callback.
//!
//! ## Scope (conservative, Mocha-specific)
//!
//! The rule is dedicated to Mocha, so detection is keyed off a call to a known
//! Mocha test/hook function whose callback declares a simple `done` parameter:
//!
//! - bare identifier callees: `it`, `test`, `specify`, `before`, `after`,
//!   `beforeEach`, `afterEach`;
//! - member callees `it.only` / `it.skip` (and the same for `test` / `specify`).
//!
//! Among the call's arguments, any inline `function`/arrow expression whose
//! parameter list contains a plain identifier parameter named `done` is treated
//! as the test callback. Restricting to Mocha call sites (rather than flagging
//! *any* function with a `done` parameter) avoids false positives on unrelated
//! error-first callbacks that conventionally use the name `done`.
//!
//! ## What counts as "after `done()`"
//!
//! Only the **top level** of the callback body is inspected: the rule looks for
//! a direct `ExpressionStatement` whose expression is a call to the bare
//! identifier `done`. It deliberately does **not** descend into nested blocks
//! (`if (x) { done(); return; }`) or nested functions, which keeps the rule
//! conservative and side-steps control-flow reasoning.
//!
//! Statements following the first such `done()` call are then examined. A
//! trailing bare `return;` (a `ReturnStatement` with no argument) is exempt —
//! it produces no observable effect and is a common, harmless way to end the
//! callback. The first remaining statement (including a `return <expr>;`, which
//! still evaluates its argument) is reported.
//!
//! Reported at the span of the offending statement.
//!
//! Behaviour is reproduced from the public RSPEC description (S6079) and the
//! public eslint-plugin-sonarjs / Mocha documentation only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `it('t', function (done) { done(); foo(); });`
//! - `beforeEach((done) => { done(); cleanup(); });`
//!
//! ## Not flagged
//! - `it('t', function (done) { foo(); done(); });` — `done()` is last
//! - `it('t', function (done) { done(); });` — nothing after `done()`
//! - `it('t', function (done) { done(); return; });` — trailing bare return
//! - `it('t', function () { foo(); });` — no `done` parameter
//! - `it('t', function (done) { if (x) { done(); foo(); } });` — nested block

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, FormalParameter, Statement,
};
use oxc_span::{GetSpan, Span};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-code-after-done";

/// Bare-identifier Mocha test/hook functions whose callback may receive `done`.
const MOCHA_FNS: [&str; 7] = [
    "it",
    "test",
    "specify",
    "before",
    "after",
    "beforeEach",
    "afterEach",
];

/// Identifiers that may carry a `.only` / `.skip` modifier (`it.only`, ...).
const MOCHA_MODIFIABLE: [&str; 3] = ["it", "test", "specify"];

/// Returns `true` if the callee names a Mocha test/hook construct.
fn is_mocha_callee(callee: &Expression<'_>) -> bool {
    match callee.get_inner_expression() {
        Expression::Identifier(id) => MOCHA_FNS.contains(&id.name.as_str()),
        Expression::StaticMemberExpression(member) => {
            let prop = member.property.name.as_str();
            if prop != "only" && prop != "skip" {
                return false;
            }
            let Expression::Identifier(object) = member.object.get_inner_expression() else {
                return false;
            };
            MOCHA_MODIFIABLE.contains(&object.name.as_str())
        }
        _ => false,
    }
}

/// Returns `true` if the parameter list contains a plain identifier named
/// `done` (destructuring / rest patterns are ignored).
fn has_done_param(params: &[FormalParameter<'_>]) -> bool {
    params.iter().any(|param| match &param.pattern {
        BindingPattern::BindingIdentifier(id) => id.name.as_str() == "done",
        _ => false,
    })
}

/// Returns `true` if `stmt` is `done();` (a call to the bare identifier `done`).
fn is_done_call(stmt: &Statement<'_>) -> bool {
    let Statement::ExpressionStatement(expr_stmt) = stmt else {
        return false;
    };
    let Expression::CallExpression(call) = expr_stmt.expression.get_inner_expression() else {
        return false;
    };
    match call.callee.get_inner_expression() {
        Expression::Identifier(id) => id.name.as_str() == "done",
        _ => false,
    }
}

/// Returns `true` if `stmt` is a bare `return;` (no argument), which is exempt.
fn is_bare_return(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ReturnStatement(ret) => ret.argument.is_none(),
        _ => false,
    }
}

/// Given the top-level statements of a callback body, returns the span of the
/// first offending statement that runs after a top-level `done()` call.
fn offending_span_after_done(stmts: &[Statement<'_>]) -> Option<Span> {
    let done_index = stmts.iter().position(is_done_call)?;
    let after = stmts.get(done_index + 1..)?;
    let offending = after.iter().find(|stmt| !is_bare_return(stmt))?;
    Some(offending.span())
}

impl Scanner<'_> {
    /// Flags the first statement after a top-level `done()` call inside a Mocha
    /// test/hook callback that declares a `done` parameter.
    pub(crate) fn check_no_code_after_done(&mut self, call: &CallExpression<'_>) {
        if !is_mocha_callee(&call.callee) {
            return;
        }
        for argument in &call.arguments {
            let Some(span) = argument_offending_span(argument) else {
                continue;
            };
            self.report(RULE_NAME, "noCodeAfterDone", span);
        }
    }
}

/// Returns the offending statement span for a call argument when it is an
/// inline function/arrow callback with a `done` parameter and a block body.
fn argument_offending_span(argument: &Argument<'_>) -> Option<Span> {
    match argument {
        Argument::FunctionExpression(func) => {
            if !has_done_param(&func.params.items) {
                return None;
            }
            let body = func.body.as_ref()?;
            offending_span_after_done(&body.statements)
        }
        Argument::ArrowFunctionExpression(arrow) => {
            if arrow.expression {
                return None;
            }
            if !has_done_param(&arrow.params.items) {
                return None;
            }
            offending_span_after_done(&arrow.body.statements)
        }
        _ => None,
    }
}
