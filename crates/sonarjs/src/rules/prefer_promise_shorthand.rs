//! Rule `prefer-promise-shorthand` (SonarJS key S4634).
//!
//! Clean-room port. Flags a `new Promise(executor)` where the executor is an
//! inline function or arrow that does nothing but call `resolve(...)` or
//! `reject(...)` immediately. Such expressions can be replaced by the simpler
//! `Promise.resolve(...)` or `Promise.reject(...)`.
//!
//! **Conditions for flagging** — all must hold:
//! 1. The callee is the bare identifier `Promise`.
//! 2. Exactly one argument, which is an `ArrowFunctionExpression` or a
//!    `FunctionExpression`.
//! 3. The executor declares 1 or 2 parameters, all simple `BindingIdentifier`s
//!    with no default values; and no rest parameter.
//! 4. The executor body is effectively a single statement: either an
//!    `ExpressionStatement` or a `ReturnStatement` wrapping a `CallExpression`.
//! 5. That call's callee is the bare identifier equal to the executor's first
//!    parameter (`resolve`) or, if present, its second parameter (`reject`).
//! 6. The call has 0 or 1 arguments (no spread elements).
//! 7. If the single call argument is an identifier equal to the OTHER
//!    executor parameter, do NOT flag (unsafe to simplify blindly).
//!
//! ## Flagged
//! - `new Promise((resolve) => resolve(42))`
//! - `new Promise((resolve) => resolve())`
//! - `new Promise((resolve, reject) => reject(err))`
//! - `new Promise(function (resolve) { resolve(1); })`
//! - `new Promise((resolve) => { resolve(1); })`
//! - `new Promise((resolve) => { return resolve(1); })`
//!
//! ## Not flagged
//! - `new Promise((resolve, reject) => { doStuff(); resolve(1); })` — multiple stmts
//! - `new Promise((resolve) => setTimeout(resolve, 100))` — not calling resolve
//! - `new Promise((resolve) => resolve(1, 2))` — two arguments to resolve
//! - `new Promise(executor)` — executor is not an inline function
//! - `foo.Promise((r) => r())` — member-expression callee, not bare `Promise`
//! - `new Promise((resolve, reject) => resolve(reject))` — arg is other param
//!
//! Behaviour is reproduced from the public RSPEC description (S4634) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, FormalParameter, FunctionBody,
    NewExpression, Statement,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-promise-shorthand";

/// Returns the parameter name if `param` is a plain `BindingIdentifier` with
/// no default value. Returns `None` for destructuring, rest, or defaults.
fn param_name<'a>(param: &'a FormalParameter) -> Option<&'a str> {
    let BindingPattern::BindingIdentifier(id) = &param.pattern else {
        return None;
    };
    if param.initializer.is_some() {
        return None;
    }
    Some(id.name.as_str())
}

/// Returns the single `CallExpression` in a function body that has exactly one
/// statement, which must be either an `ExpressionStatement` wrapping a call or
/// a `ReturnStatement` whose argument is a call. Returns `None` otherwise.
fn single_call_in_body<'a>(body: &'a FunctionBody<'a>) -> Option<&'a CallExpression<'a>> {
    let [stmt] = body.statements.as_slice() else {
        return None;
    };
    match stmt {
        Statement::ExpressionStatement(e_stmt) => match e_stmt.expression.get_inner_expression() {
            Expression::CallExpression(call) => Some(call),
            _ => None,
        },
        Statement::ReturnStatement(ret) => {
            let arg = ret.argument.as_ref()?;
            match arg.get_inner_expression() {
                Expression::CallExpression(call) => Some(call),
                _ => None,
            }
        }
        _ => None,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_prefer_promise_shorthand(&mut self, expr: &NewExpression<'_>) {
        // 1. Callee must be the bare identifier `Promise`.
        let Expression::Identifier(callee) = expr.callee.get_inner_expression() else {
            return;
        };
        if callee.name.as_str() != "Promise" {
            return;
        }

        // 2. Exactly one argument that is an inline function.
        if expr.arguments.len() != 1 {
            return;
        }
        let (params, body) = match &expr.arguments[0] {
            Argument::ArrowFunctionExpression(arrow) => {
                (arrow.params.as_ref(), arrow.body.as_ref())
            }
            Argument::FunctionExpression(func) => {
                let Some(b) = func.body.as_ref() else {
                    return;
                };
                (func.params.as_ref(), b.as_ref())
            }
            _ => return,
        };

        // 3. 1 or 2 simple params, no rest, no defaults.
        if params.items.is_empty() || params.items.len() > 2 {
            return;
        }
        if params.rest.is_some() {
            return;
        }
        let Some(resolve_name) = param_name(&params.items[0]) else {
            return;
        };
        let reject_name = if params.items.len() == 2 {
            let Some(n) = param_name(&params.items[1]) else {
                return;
            };
            Some(n)
        } else {
            None
        };

        // 4. Body must be a single call expression.
        let Some(call) = single_call_in_body(body) else {
            return;
        };

        // 5. Call's callee must be the resolve or reject identifier.
        let Expression::Identifier(call_callee) = call.callee.get_inner_expression() else {
            return;
        };
        let callee_name = call_callee.name.as_str();
        if callee_name != resolve_name && Some(callee_name) != reject_name {
            return;
        }

        // 6. Call has 0 or 1 arguments, no spread.
        if call.arguments.len() > 1 {
            return;
        }
        if call.arguments.first().map(Argument::is_spread) == Some(true) {
            return;
        }

        // 7. If the single argument is the other parameter, do not flag.
        let other_param = if callee_name == resolve_name {
            reject_name
        } else {
            Some(resolve_name)
        };
        let arg_is_other = match (other_param, call.arguments.first()) {
            (Some(other_name), Some(Argument::Identifier(id))) => id.name.as_str() == other_name,
            _ => false,
        };
        if arg_is_other {
            return;
        }

        self.report(RULE_NAME, "preferShorthand", expr.span);
    }
}
