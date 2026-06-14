//! Rule `prefer-immediate-return` (SonarJS key S1488).
//!
//! Clean-room port. Declaring a local variable only to immediately `return` or
//! `throw` it on the next statement is unnecessary indirection — return or
//! throw the expression directly instead.
//!
//! The rule fires on a `FunctionBody` whose **last two statements** are:
//!
//! 1. A `VariableDeclaration` with **exactly one** declarator whose binding is
//!    a simple `BindingIdentifier` (name N) and whose `init` is `Some(_)`.
//! 2. Either a `ReturnStatement` with `argument` that resolves (after stripping
//!    parentheses via `get_inner_expression`) to an `Identifier` named N, or a
//!    `ThrowStatement` whose `argument` resolves to an `Identifier` named N.
//!
//! The reported span is the span of the last statement (the return/throw).
//!
//! ## Scope
//!
//! Only `FunctionBody` nodes are checked (function declarations, function
//! expressions, methods, and arrow functions with block bodies). Non-function
//! bare blocks (`{ … }`, `if` bodies, etc.) are **out of scope** for this
//! check — detecting them requires distinguishing block-statement bodies from
//! function bodies, which is left as a follow-up.
//!
//! ## Flagged
//!
//! ```js
//! function f() { const x = compute(); return x; }
//! function f() { let y = a + b; return y; }
//! function f() { const e = new Error(); throw e; }
//! const g = () => { const x = 1; return x; };
//! ```
//!
//! ## Not flagged
//!
//! ```js
//! // only one statement — direct return, nothing to flag
//! function f() { return compute(); }
//! // extra statements between the declaration and the return
//! function f() { const x = 1; doStuff(); return x; }
//! // returns a different identifier
//! function f() { const x = 1; return y; }
//! // two declarators
//! function f() { const x = 1, y = 2; return x; }
//! // no init — `let x;` declares without a value
//! function f() { let x; return x; }
//! // returns an expression, not the bare identifier
//! function f() { const x = 1; return x + 1; }
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{BindingPattern, Expression, FunctionBody, Statement};
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-immediate-return";

/// If `stmt` is a `VariableDeclaration` with exactly one declarator that has a
/// simple `BindingIdentifier` id and a non-`None` init, returns the bound name.
/// Otherwise returns `None`.
fn declared_single_name<'a>(stmt: &'a Statement) -> Option<&'a str> {
    let Statement::VariableDeclaration(decl) = stmt else {
        return None;
    };
    if decl.declarations.len() != 1 {
        return None;
    }
    let declarator = &decl.declarations[0];
    declarator.init.as_ref()?;
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some(id.name.as_str())
}

/// If `stmt` is a `ReturnStatement` or `ThrowStatement` whose argument
/// (after stripping parentheses) is a plain `Identifier`, returns that name.
/// Otherwise returns `None`.
fn returned_or_thrown_name<'a>(stmt: &'a Statement) -> Option<&'a str> {
    match stmt {
        Statement::ReturnStatement(ret) => {
            let expr = ret.argument.as_ref()?.get_inner_expression();
            let Expression::Identifier(id) = expr else {
                return None;
            };
            Some(id.name.as_str())
        }
        Statement::ThrowStatement(thr) => {
            let expr = thr.argument.get_inner_expression();
            let Expression::Identifier(id) = expr else {
                return None;
            };
            Some(id.name.as_str())
        }
        _ => None,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_prefer_immediate_return(&mut self, body: &FunctionBody<'_>) {
        let [.., second_last, last] = body.statements.as_slice() else {
            return;
        };
        let Some(decl_name) = declared_single_name(second_last) else {
            return;
        };
        let Some(ret_name) = returned_or_thrown_name(last) else {
            return;
        };
        if decl_name != ret_name {
            return;
        }
        self.report(RULE_NAME, "preferImmediateReturn", last.span());
    }
}
