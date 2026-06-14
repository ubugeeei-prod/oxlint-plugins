//! Rule `array-callback-without-return` (SonarJS key S3796).
//!
//! Clean-room port. Several `Array` methods build their result *from the value
//! returned by their callback* (`map` collects each return, `filter`/`find`/
//! `some`/`every` test it, `reduce` threads it as the accumulator, `sort` uses
//! it as a comparison, etc.). A callback passed to one of these methods that
//! never returns a value is almost always a bug: the array is silently filled
//! with `undefined`, every predicate sees a falsy value, the accumulator is
//! lost. This rule flags such a callback.
//!
//! ## Covered methods (the value-consuming ones)
//!
//! `map`, `filter`, `every`, `some`, `find`, `findIndex`, `findLast`,
//! `findLastIndex`, `reduce`, `reduceRight`, `flatMap`, `sort`.
//!
//! For every covered method the callback is the **first** argument (this holds
//! for `reduce`/`reduceRight` too — their optional initial value is the second
//! argument). `forEach` is deliberately **not** covered: its callback's return
//! value is intentionally ignored. `Array.from`/`Array.fromAsync` are also not
//! covered: their map function is the *second* argument and they are static
//! calls on `Array`, so handling them would require a separate code path; they
//! are omitted to keep this port conservative.
//!
//! ## Receiver is not type-checked
//!
//! Like ESLint's `array-callback-return`, the rule keys purely off the method
//! **name** on any receiver — no proof that the receiver is an array is
//! required (the method names are distinctive). A non-array receiver that
//! happens to expose a same-named method (e.g. a custom `obj.filter(cb)`) is a
//! rare, accepted over-report. This matches the established type-free
//! convention in this crate.
//!
//! ## "Does not return a value" — conservative (any-return) semantics
//!
//! ESLint's analogous rule requires a value to be returned on *all* code paths.
//! To avoid false positives on partial-return callbacks, this port implements
//! the conservative variant: a callback is flagged **only** when its block body
//! contains **zero** `return <expr>` statements anywhere in the body's
//! non-nested-function subtree. A bare `return;` (no argument) does not count as
//! returning a value. If any value-returning `return` exists, the callback is
//! left alone, so partial-return callbacks are never reported (a documented,
//! intentional under-report relative to the all-paths-return interpretation).
//!
//! An arrow with an **expression body** (`x => x + 1`) implicitly returns its
//! expression and is therefore never flagged. A callback that is not an inline
//! function/arrow (e.g. a bare identifier `arr.map(fn)`) cannot be inspected and
//! is never flagged.
//!
//! ## Body walk that stops at nested functions
//!
//! A `return` always binds to its nearest enclosing function/arrow, so the walk
//! descends through control-flow statements (block, if/else, the loop bodies,
//! `with`, labeled, switch cases, `try`/`catch`/`finally`) but **never** into a
//! nested `FunctionDeclaration`, function expression, or arrow — those returns
//! belong to the nested scope, not to our callback. `return` is only ever a
//! statement, so expression positions need not be traversed at all.
//!
//! Reported at the callback function's span.
//!
//! Behaviour is reproduced from the public RSPEC description (S3796) and the
//! public eslint-plugin-sonarjs / ESLint rule documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `[1, 2].map(function (x) { console.log(x); });` — block body, no return
//! - `arr.filter((x) => { doStuff(x); });` — block body, no value return
//!
//! ## Not flagged
//! - `[1, 2].map((x) => x + 1);` — expression body, implicit return
//! - `arr.filter(function (x) { return x > 0; });` — has a value return
//! - `arr.forEach((x) => { log(x); });` — `forEach` is not covered
//! - `arr.map(fn);` — callback is an identifier, not an inline function

use oxc_ast::ast::{Argument, CallExpression, Expression, Statement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "array-callback-without-return";

/// Array methods whose result is built from the callback's return value.
const COVERED_METHODS: [&str; 12] = [
    "map",
    "filter",
    "every",
    "some",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "reduce",
    "reduceRight",
    "flatMap",
    "sort",
];

/// Returns `true` if any statement in `stmts` contains a value-returning
/// `return` within its non-nested-function subtree.
fn statements_return_value(stmts: &[Statement<'_>]) -> bool {
    stmts.iter().any(statement_returns_value)
}

/// Returns `true` if `stmt`'s subtree contains a `return <expr>;`, descending
/// through control-flow statements but never into a nested function/arrow.
fn statement_returns_value(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ReturnStatement(ret) => ret.argument.is_some(),
        Statement::BlockStatement(block) => statements_return_value(&block.body),
        Statement::IfStatement(node) => {
            statement_returns_value(&node.consequent)
                || node.alternate.as_ref().is_some_and(statement_returns_value)
        }
        Statement::ForStatement(node) => statement_returns_value(&node.body),
        Statement::ForInStatement(node) => statement_returns_value(&node.body),
        Statement::ForOfStatement(node) => statement_returns_value(&node.body),
        Statement::WhileStatement(node) => statement_returns_value(&node.body),
        Statement::DoWhileStatement(node) => statement_returns_value(&node.body),
        Statement::WithStatement(node) => statement_returns_value(&node.body),
        Statement::LabeledStatement(node) => statement_returns_value(&node.body),
        Statement::SwitchStatement(node) => node
            .cases
            .iter()
            .any(|case| statements_return_value(&case.consequent)),
        Statement::TryStatement(node) => try_returns_value(node.as_ref()),
        _ => false,
    }
}

/// Returns `true` if any of a `try`'s block, handler, or finalizer returns a
/// value. Split out to keep [`statement_returns_value`] within line width.
fn try_returns_value(node: &oxc_ast::ast::TryStatement<'_>) -> bool {
    statements_return_value(&node.block.body)
        || node
            .handler
            .as_ref()
            .is_some_and(|handler| statements_return_value(&handler.body.body))
        || node
            .finalizer
            .as_ref()
            .is_some_and(|finalizer| statements_return_value(&finalizer.body))
}

impl<'a> Scanner<'a> {
    /// Flags a call to a value-consuming array method whose inline callback
    /// (first argument) never returns a value.
    pub(crate) fn check_array_callback_without_return(&mut self, call: &CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        if !COVERED_METHODS.contains(&member.property.name.as_str()) {
            return;
        }
        let Some(argument) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let span = match argument.get_inner_expression() {
            Expression::FunctionExpression(func) => {
                let Some(body) = func.body.as_ref() else {
                    return;
                };
                if statements_return_value(&body.statements) {
                    return;
                }
                func.span
            }
            Expression::ArrowFunctionExpression(arrow) => {
                if arrow.expression {
                    return;
                }
                if statements_return_value(&arrow.body.statements) {
                    return;
                }
                arrow.span
            }
            _ => return,
        };
        self.report(RULE_NAME, "addReturn", span);
    }
}
