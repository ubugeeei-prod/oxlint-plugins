//! Rule `function-return-type` (SonarJS key S3800).
//!
//! Clean-room port. A function that can return values of several different
//! types is harder to read and to use correctly than one that always returns a
//! single type. SonarJS flags functions whose set of returned types contains
//! more than one "real" type, while deliberately tolerating `null`/`undefined`
//! (a nullable single type stays compliant).
//!
//! ## Narrow form
//!
//! Faithfully reproducing the full rule needs a TypeScript type checker to infer
//! the type of every returned expression, which this runtime does not have. This
//! port therefore enforces the unambiguous, type-checker-independent core: it
//! only inspects `return` statements whose argument is a *primitive literal*
//! whose category is syntactically certain — a string (or template) literal, a
//! numeric literal, a boolean literal, or a bigint literal. When a single
//! function body explicitly returns literals of two or more *different*
//! categories, the conflict is real regardless of any type information:
//!
//! ```js
//! function f(x) {        // Noncompliant: returns a number and a string
//!   if (x) return 1;
//!   return "a";
//! }
//! ```
//!
//! Returns whose argument is anything else (identifiers, calls, objects,
//! arithmetic, `null`, `undefined`, …) are ignored, so the check never guesses a
//! type and never produces a false positive. Returns inside nested functions or
//! arrows belong to those inner scopes and are not mixed in (each function is
//! visited and checked on its own). Functions with fewer than two distinct
//! literal categories are compliant. This intentionally under-reports relative to
//! the type-aware SonarJS rule.
//!
//! Behaviour is reproduced from the public RSPEC description (S3800) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, FunctionBody, Statement};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "function-return-type";

/// Classifies a returned expression into a syntactically certain primitive
/// category, or `None` when the type cannot be known without inference.
fn literal_category(expr: &Expression<'_>) -> Option<&'static str> {
    match expr {
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => Some("string"),
        Expression::NumericLiteral(_) => Some("number"),
        Expression::BooleanLiteral(_) => Some("boolean"),
        Expression::BigIntLiteral(_) => Some("bigint"),
        _ => None,
    }
}

/// Collects the literal categories of every `return <expr>;` reachable from
/// `stmts` without crossing into a nested function or arrow scope.
fn collect_in_statements<'a>(stmts: &[Statement<'a>], out: &mut SmallVec<[&'static str; 4]>) {
    for stmt in stmts {
        collect_in_statement(stmt, out);
    }
}

fn collect_in_statement<'a>(stmt: &Statement<'a>, out: &mut SmallVec<[&'static str; 4]>) {
    match stmt {
        Statement::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument
                && let Some(category) = literal_category(arg)
            {
                out.push(category);
            }
        }
        Statement::BlockStatement(block) => collect_in_statements(&block.body, out),
        Statement::IfStatement(if_stmt) => {
            collect_in_statement(&if_stmt.consequent, out);
            if let Some(alt) = &if_stmt.alternate {
                collect_in_statement(alt, out);
            }
        }
        Statement::ForStatement(for_stmt) => collect_in_statement(&for_stmt.body, out),
        Statement::ForInStatement(for_stmt) => collect_in_statement(&for_stmt.body, out),
        Statement::ForOfStatement(for_stmt) => collect_in_statement(&for_stmt.body, out),
        Statement::WhileStatement(while_stmt) => collect_in_statement(&while_stmt.body, out),
        Statement::DoWhileStatement(do_stmt) => collect_in_statement(&do_stmt.body, out),
        Statement::LabeledStatement(labeled) => collect_in_statement(&labeled.body, out),
        Statement::WithStatement(with_stmt) => collect_in_statement(&with_stmt.body, out),
        Statement::SwitchStatement(switch_stmt) => {
            for case in &switch_stmt.cases {
                collect_in_statements(&case.consequent, out);
            }
        }
        Statement::TryStatement(try_stmt) => {
            collect_in_statements(&try_stmt.block.body, out);
            if let Some(handler) = &try_stmt.handler {
                collect_in_statements(&handler.body.body, out);
            }
            if let Some(finalizer) = &try_stmt.finalizer {
                collect_in_statements(&finalizer.body, out);
            }
        }
        // All other statements either contain no nested statements relevant to
        // this scope (return statements never live inside an expression) or open
        // a new function/class scope that is visited and checked separately.
        _ => {}
    }
}

impl Scanner<'_> {
    pub(crate) fn check_function_return_type(&mut self, body: &FunctionBody<'_>, span: Span) {
        let mut categories: SmallVec<[&'static str; 4]> = SmallVec::new();
        collect_in_statements(&body.statements, &mut categories);

        // Count distinct categories without allocating a set.
        let mut distinct: SmallVec<[&'static str; 4]> = SmallVec::new();
        for category in &categories {
            if !distinct.contains(category) {
                distinct.push(category);
            }
        }

        if distinct.len() >= 2 {
            self.report(RULE_NAME, "differentTypes", span);
        }
    }
}
