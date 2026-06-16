//! Rule `no-async-constructor` (SonarJS key S7059).
//!
//! Clean-room port. A class constructor must initialize the instance
//! synchronously. Kicking off a promise from the constructor means the instance
//! is handed back to the caller before the asynchronous work completes, leaving
//! the properties that work was meant to populate still `undefined`. The
//! asynchronous work belongs in an `async` method or a static factory method
//! instead.
//!
//! ## Zero-false-positive subset
//!
//! Only the **direct, top-level statements** of the constructor body are
//! inspected; the check never descends into nested function, arrow-function, or
//! class bodies. For each top-level statement, the "primary" expression(s) are
//! extracted:
//!
//! - an `ExpressionStatement` — its expression, plus the right-hand side when
//!   that expression is an assignment (`this.data = Promise...().then(...)`);
//! - a `VariableDeclaration` — the initializer of each declarator
//!   (`const p = fetchData().then(...)`);
//! - a `ReturnStatement` — its argument.
//!
//! Other statement kinds (`if`, `for`, ...) are intentionally ignored to stay
//! conservative; under-reporting is acceptable, false positives are not.
//!
//! An extracted expression is an "async operation call" when, after
//! `get_inner_expression()`, it is a `CallExpression` whose callee (also
//! unwrapped) is a `StaticMemberExpression` that is either:
//!
//! - consuming a promise — its property is `then`, `catch`, or `finally`
//!   (`Promise.resolve().then(...)`, `fetchData().then(...)`); or
//! - producing one — its object is the identifier `Promise` and its property is
//!   one of `resolve`, `reject`, `all`, `allSettled`, `race`, `any`
//!   (`Promise.all([...])` used as a statement).
//!
//! Because nested callbacks are never visited, a constructor that merely
//! *defines* an asynchronous handler is not flagged: in
//! `this.handler = () => fetch().then(() => {})` the `.then` lives inside a
//! nested arrow, so the top-level statement contains no async-op call.
//!
//! **Flagged**:
//! ```js
//! class MyClass {
//!   constructor() {
//!     Promise.resolve().then(() => this.data = fetchData()); // top-level .then
//!   }
//! }
//! ```
//!
//! **Not flagged**:
//! ```js
//! class MyClass {
//!   constructor() { this.data = null; }            // synchronous init
//!   async initialize() { this.data = await fetchData(); } // async work in a method
//! }
//! class MyClass2 {
//!   constructor() { this.handler = () => fetch().then(() => {}); } // .then nested in arrow
//! }
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description (S7059) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Class, ClassElement, Expression, MethodDefinitionKind, Statement};
use oxc_span::{GetSpan, Span};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-async-constructor";

/// Returns `true` when `expr`, after unwrapping parentheses/casts, is a
/// `CallExpression` that either consumes a promise (`.then`/`.catch`/`.finally`)
/// or produces one via a `Promise.<combinator>(...)` static call.
fn is_async_op_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr.get_inner_expression() else {
        return false;
    };
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    let property = member.property.name.as_str();
    if matches!(property, "then" | "catch" | "finally") {
        return true;
    }
    let Expression::Identifier(object) = member.object.get_inner_expression() else {
        return false;
    };
    object.name == "Promise"
        && matches!(
            property,
            "resolve" | "reject" | "all" | "allSettled" | "race" | "any"
        )
}

impl Scanner<'_> {
    pub(crate) fn check_no_async_constructor(&mut self, it: &Class<'_>) {
        for element in &it.body.body {
            let ClassElement::MethodDefinition(method) = element else {
                continue;
            };
            if method.kind != MethodDefinitionKind::Constructor {
                continue;
            }
            let Some(body) = &method.value.body else {
                continue;
            };
            for statement in &body.statements {
                if let Some(span) = offending_async_op_span(statement) {
                    self.report(RULE_NAME, "noAsyncConstructor", span);
                    return;
                }
            }
        }
    }
}

/// Extracts the primary expression(s) of a single top-level constructor
/// statement and returns the span of the first async-op call found, without
/// descending into nested function/arrow/class bodies.
fn offending_async_op_span(statement: &Statement<'_>) -> Option<Span> {
    match statement {
        Statement::ExpressionStatement(es) => {
            if is_async_op_call(&es.expression) {
                return Some(es.expression.span());
            }
            let Expression::AssignmentExpression(assign) = es.expression.get_inner_expression()
            else {
                return None;
            };
            is_async_op_call(&assign.right).then(|| assign.right.span())
        }
        Statement::VariableDeclaration(decl) => {
            for declarator in &decl.declarations {
                let Some(init) = &declarator.init else {
                    continue;
                };
                if is_async_op_call(init) {
                    return Some(init.span());
                }
            }
            None
        }
        Statement::ReturnStatement(ret) => {
            let argument = ret.argument.as_ref()?;
            is_async_op_call(argument).then(|| argument.span())
        }
        _ => None,
    }
}
