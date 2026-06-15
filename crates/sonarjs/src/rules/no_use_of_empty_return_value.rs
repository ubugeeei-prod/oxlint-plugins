//! Rule `no-use-of-empty-return-value` (SonarJS key S3699).
//!
//! Clean-room port. A function that never explicitly returns a value always
//! yields `undefined` at every call site. Consuming that return value (storing
//! it in a variable, assigning it to a property, or re-returning it) is almost
//! certainly a bug — the programmer probably forgot to add a `return` statement.
//!
//! ## What makes a function "void"
//!
//! A function is void when **all** of the following hold:
//! - It is **not** `async` (async functions always return a `Promise`).
//! - It is **not** a generator (generators return an iterator object).
//! - It is **not** an expression-bodied arrow (`() => expr` implicitly returns
//!   the expression).
//! - Its body contains **no** `return <expression>;` statement at any depth
//!   inside the function, stopping the scan at nested function/arrow boundaries
//!   (because a `return` always binds to its nearest enclosing scope).
//!
//! ## Consumer positions that are flagged
//!
//! - `VariableDeclarator` whose `init` is a `CallExpression` targeting a void
//!   function (`const x = voidFn();`).
//! - Plain `AssignmentExpression` (`=`) whose right-hand side is such a call
//!   (`x = voidFn();`). Compound assignments (`+=`, etc.) are not flagged.
//! - `ReturnStatement` whose argument is such a call (`return voidFn();`).
//!
//! A bare `voidFn();` as an `ExpressionStatement` is **not** flagged: the
//! caller explicitly discards the value, which is the correct idiom.
//!
//! ## Zero-false-positive guards
//!
//! - Only identifier callees that can be resolved through semantic analysis to a
//!   function definition in the same file are checked. Unresolvable callees (e.g.
//!   calls through dynamic expressions, imported functions, or mutated bindings)
//!   are silently skipped.
//! - Requires semantic analysis; nothing is emitted when semantic data is absent.
//!
//! Behaviour is reproduced from the public RSPEC description (S3699,
//! "The return value of void functions should not be used") only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    AssignmentExpression, CallExpression, Expression, FunctionBody, ReturnStatement, Statement,
    VariableDeclarator,
};
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-use-of-empty-return-value";

// ----- Body-walk helpers ----------------------------------------------------

/// Returns `true` when any statement in `stmts` contains a value-returning
/// `return` within its non-nested-function subtree.
fn statements_have_value_return(stmts: &[Statement<'_>]) -> bool {
    stmts.iter().any(stmt_has_value_return)
}

/// Returns `true` when `stmt` — or any statement reachable from it that is
/// **not** separated by a function/arrow boundary — contains a `return <expr>;`.
fn stmt_has_value_return(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ReturnStatement(ret) => ret.argument.is_some(),
        Statement::BlockStatement(block) => statements_have_value_return(&block.body),
        Statement::IfStatement(node) => {
            stmt_has_value_return(&node.consequent)
                || node.alternate.as_ref().is_some_and(stmt_has_value_return)
        }
        Statement::ForStatement(node) => stmt_has_value_return(&node.body),
        Statement::ForInStatement(node) => stmt_has_value_return(&node.body),
        Statement::ForOfStatement(node) => stmt_has_value_return(&node.body),
        Statement::WhileStatement(node) => stmt_has_value_return(&node.body),
        Statement::DoWhileStatement(node) => stmt_has_value_return(&node.body),
        Statement::WithStatement(node) => stmt_has_value_return(&node.body),
        Statement::LabeledStatement(node) => stmt_has_value_return(&node.body),
        Statement::SwitchStatement(node) => node
            .cases
            .iter()
            .any(|case| statements_have_value_return(&case.consequent)),
        Statement::TryStatement(node) => try_stmt_has_value_return(node.as_ref()),
        // FunctionDeclaration / FunctionExpression / ArrowFunctionExpression:
        // their `return` statements belong to that nested scope, not ours.
        // All other statement kinds contain no `return` statements.
        _ => false,
    }
}

/// Returns `true` when any of a `try` block, handler, or finalizer contains a
/// value-returning `return`. Split out to keep [`stmt_has_value_return`] within
/// reasonable line width.
fn try_stmt_has_value_return(node: &oxc_ast::ast::TryStatement<'_>) -> bool {
    statements_have_value_return(&node.block.body)
        || node
            .handler
            .as_ref()
            .is_some_and(|handler| statements_have_value_return(&handler.body.body))
        || node
            .finalizer
            .as_ref()
            .is_some_and(|finalizer| statements_have_value_return(&finalizer.body))
}

/// Returns `true` when `body` is considered "void": it has no `return <expr>;`
/// at any reachable depth (stopping at nested function/arrow boundaries).
fn body_is_void(body: &FunctionBody<'_>) -> bool {
    !statements_have_value_return(&body.statements)
}

// ----- Scanner impl ---------------------------------------------------------

impl<'a> Scanner<'a> {
    // ------ Public entry points called from scanner.rs ----------------------

    /// Checks `const x = voidFn();` and `let y = voidFn();`.
    pub(crate) fn check_no_use_of_empty_return_value_var(&mut self, decl: &VariableDeclarator<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let init = match &decl.init {
            Some(e) => e,
            None => return,
        };
        let call = match init.get_inner_expression() {
            Expression::CallExpression(call) => call,
            _ => return,
        };
        if self.is_void_call(call) {
            self.report(RULE_NAME, "useOfEmptyReturnValue", call.span);
        }
    }

    /// Checks `x = voidFn();` (plain `=` only; compound assignments are skipped
    /// because their semantic is different from a simple capture of the value).
    pub(crate) fn check_no_use_of_empty_return_value_assign(
        &mut self,
        expr: &AssignmentExpression<'a>,
    ) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        if !matches!(expr.operator, AssignmentOperator::Assign) {
            return;
        }
        let call = match expr.right.get_inner_expression() {
            Expression::CallExpression(call) => call,
            _ => return,
        };
        if self.is_void_call(call) {
            self.report(RULE_NAME, "useOfEmptyReturnValue", call.span);
        }
    }

    /// Checks `return voidFn();`.
    pub(crate) fn check_no_use_of_empty_return_value_return(&mut self, stmt: &ReturnStatement<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let arg = match &stmt.argument {
            Some(e) => e,
            None => return,
        };
        let call = match arg.get_inner_expression() {
            Expression::CallExpression(call) => call,
            _ => return,
        };
        if self.is_void_call(call) {
            self.report(RULE_NAME, "useOfEmptyReturnValue", call.span);
        }
    }

    // ------ Private resolution helpers -------------------------------------

    /// Returns `true` when `call`'s callee resolves (via semantic analysis) to a
    /// void function definition in the current file.
    fn is_void_call(&self, call: &CallExpression<'a>) -> bool {
        let ident = match call.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident,
            _ => return false,
        };

        // Path 1 — callee is a `const`/`let`/`var`-bound function expression or
        // arrow (`const f = () => { … }`). `resolve_identifier_initializer` only
        // succeeds for a non-mutated binding whose declaration site is a
        // `VariableDeclarator`, so import bindings and function declarations are
        // handled by Path 2.
        if let Some(init_expr) = self.resolve_identifier_initializer(ident) {
            return match init_expr.get_inner_expression() {
                Expression::FunctionExpression(func) => {
                    if func.r#async || func.generator {
                        return false;
                    }
                    match &func.body {
                        Some(body) => body_is_void(body),
                        None => false,
                    }
                }
                Expression::ArrowFunctionExpression(arrow) => {
                    if arrow.r#async {
                        return false;
                    }
                    // Expression-bodied arrow `() => expr` always returns the
                    // expression; it is never void.
                    if arrow.expression {
                        return false;
                    }
                    body_is_void(&arrow.body)
                }
                _ => false,
            };
        }

        // Path 2 — callee is a function declaration (`function f() { … }`).
        // Requires scoping + node data from semantic analysis.
        let scoping = match self.scoping {
            Some(s) => s,
            None => return false,
        };
        let nodes = match self.nodes {
            Some(n) => n,
            None => return false,
        };
        let reference_id = match ident.reference_id.get() {
            Some(id) => id,
            None => return false,
        };
        let symbol_id = match scoping.get_reference(reference_id).symbol_id() {
            Some(id) => id,
            None => return false,
        };
        // A reassigned function binding may no longer point at this declaration,
        // so its body is not authoritative — skip to avoid a false positive.
        if scoping.symbol_is_mutated(symbol_id) {
            return false;
        }
        let decl_node_id = scoping.symbol_declaration(symbol_id);
        let func = match nodes.get_node(decl_node_id).kind() {
            AstKind::Function(f) => f,
            _ => return false,
        };
        if func.r#async || func.generator {
            return false;
        }
        match &func.body {
            Some(body) => body_is_void(body),
            None => false,
        }
    }
}
