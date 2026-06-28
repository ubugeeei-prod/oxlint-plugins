//! Rule `no-selector-parameter` (SonarJS key S2301).
//!
//! Clean-room port. A "selector" parameter is a boolean argument whose only
//! purpose is to choose, inside the function body, which of two behaviours the
//! function performs. Such a parameter couples two responsibilities into a
//! single function, makes call sites unreadable (`render(true)` says nothing),
//! and is a hint that the function should be split into two clearly named
//! functions instead.
//!
//! ## Narrow form
//!
//! This port flags a parameter only when BOTH of the following hold, which is
//! the unambiguous, configuration-independent core of the RSPEC description and
//! guarantees no false positives:
//!
//! 1. The parameter is *syntactically* a boolean — its binding is a plain
//!    identifier (not a destructuring/rest pattern) and it either has a boolean
//!    literal default (`flag = true` / `flag = false`) or a bare `: boolean`
//!    TypeScript type annotation. No type inference is attempted.
//! 2. That same identifier is used *directly* as the test of an `if` statement
//!    inside the function body — either `if (flag)` or `if (!flag)` (parentheses
//!    are seen through). The body is scanned through nested blocks, loops,
//!    labels, `try`/`catch`/`finally`, and `switch` cases, but NOT into nested
//!    function/arrow bodies, so a parameter cannot be confused with a
//!    same-named binding in an inner scope.
//!
//! ```js
//! function render(highlight: boolean) {   // Noncompliant: `highlight`
//!   if (highlight) { drawA(); } else { drawB(); }
//! }
//! function setState(enabled = false) {     // Noncompliant: `enabled`
//!   if (!enabled) { reset(); }
//! }
//! function area(width, height) { ... }     // Compliant: no boolean selector
//! ```
//!
//! Compound conditions (`if (flag && other)`), uses outside an `if` test, and
//! parameters whose boolean-ness can only be established by inference are
//! intentionally out of scope — the port under-reports rather than risk a false
//! positive. Both concrete functions/methods (`visit_function`) and block-bodied
//! arrow functions (`visit_arrow_function_expression`) are covered; expression
//! arrows have no statement body and are skipped.
//!
//! Identifier matching is done on the source text of the binding versus the
//! source text of the `if`-test identifier, which avoids any reliance on
//! arena-lifetime details of the AST string type.
//!
//! Behaviour is reproduced from the public RSPEC description (S2301) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, Expression, FormalParameter, FormalParameters,
    Function, Statement, TSType,
};
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-selector-parameter";

impl<'a> Scanner<'a> {
    /// Entry for concrete functions and methods. Bodiless nodes (overloads,
    /// abstract/ambient signatures) have no statements to inspect and are
    /// skipped automatically.
    pub(crate) fn check_no_selector_parameter(&mut self, func: &Function<'a>) {
        let Some(body) = &func.body else {
            return;
        };
        self.report_selector_parameters(&func.params, &body.statements);
    }

    /// Entry for block-bodied arrow functions. Expression-bodied arrows have no
    /// `if` statement in their body, so they are skipped.
    pub(crate) fn check_no_selector_parameter_arrow(
        &mut self,
        arrow: &ArrowFunctionExpression<'a>,
    ) {
        if arrow.expression {
            return;
        }
        self.report_selector_parameters(&arrow.params, &arrow.body.statements);
    }

    fn report_selector_parameters(
        &mut self,
        params: &FormalParameters<'a>,
        statements: &[Statement<'a>],
    ) {
        // (identifier span, full-parameter span) for each syntactically-boolean
        // plain-identifier parameter.
        let mut bool_params: SmallVec<[(Span, Span); 4]> = SmallVec::new();
        for param in &params.items {
            let BindingPattern::BindingIdentifier(id) = &param.pattern else {
                continue;
            };
            if !is_boolean_param(param) {
                continue;
            }
            bool_params.push((id.span, param.span));
        }
        if bool_params.is_empty() {
            return;
        }

        // Spans of every `if (x)` / `if (!x)` test identifier in the body.
        let mut used: SmallVec<[Span; 8]> = SmallVec::new();
        collect_selector_uses(statements, &mut used);
        if used.is_empty() {
            return;
        }

        for (name_span, param_span) in bool_params {
            let name = self.text(name_span);
            let is_selector = used.iter().any(|test_span| self.text(*test_span) == name);
            if is_selector {
                self.report(RULE_NAME, "selectorParameter", param_span);
            }
        }
    }
}

/// A parameter is treated as boolean when it has a boolean literal default or a
/// bare `: boolean` type annotation. No type inference is performed.
fn is_boolean_param(param: &FormalParameter<'_>) -> bool {
    if matches!(
        param.initializer.as_deref(),
        Some(Expression::BooleanLiteral(_))
    ) {
        return true;
    }
    if let Some(annotation) = &param.type_annotation
        && matches!(annotation.type_annotation, TSType::TSBooleanKeyword(_))
    {
        return true;
    }
    false
}

/// Walks `statements`, recording the span of the base identifier of every `if`
/// test of the form `if (name)` / `if (!name)`. Descends through statement-level
/// nesting but never into nested function/arrow bodies (those live inside
/// expressions, which are not traversed here).
fn collect_selector_uses(statements: &[Statement<'_>], used: &mut SmallVec<[Span; 8]>) {
    for statement in statements {
        collect_in_statement(statement, used);
    }
}

fn collect_in_statement(statement: &Statement<'_>, used: &mut SmallVec<[Span; 8]>) {
    match statement {
        Statement::IfStatement(stmt) => {
            if let Some(span) = selector_test_span(&stmt.test) {
                used.push(span);
            }
            collect_in_statement(&stmt.consequent, used);
            if let Some(alternate) = &stmt.alternate {
                collect_in_statement(alternate, used);
            }
        }
        Statement::BlockStatement(stmt) => collect_selector_uses(&stmt.body, used),
        Statement::ForStatement(stmt) => collect_in_statement(&stmt.body, used),
        Statement::ForInStatement(stmt) => collect_in_statement(&stmt.body, used),
        Statement::ForOfStatement(stmt) => collect_in_statement(&stmt.body, used),
        Statement::WhileStatement(stmt) => collect_in_statement(&stmt.body, used),
        Statement::DoWhileStatement(stmt) => collect_in_statement(&stmt.body, used),
        Statement::LabeledStatement(stmt) => collect_in_statement(&stmt.body, used),
        Statement::TryStatement(stmt) => {
            collect_selector_uses(&stmt.block.body, used);
            if let Some(handler) = &stmt.handler {
                collect_selector_uses(&handler.body.body, used);
            }
            if let Some(finalizer) = &stmt.finalizer {
                collect_selector_uses(&finalizer.body, used);
            }
        }
        Statement::SwitchStatement(stmt) => {
            for case in &stmt.cases {
                collect_selector_uses(&case.consequent, used);
            }
        }
        _ => {}
    }
}

/// Returns the span of a test that is exactly `name` or `!name`, seeing through
/// any number of redundant parentheses; `None` for any other (compound, member,
/// call, …) test.
fn selector_test_span(expr: &Expression<'_>) -> Option<Span> {
    match expr {
        Expression::Identifier(ident) => Some(ident.span),
        Expression::ParenthesizedExpression(paren) => selector_test_span(&paren.expression),
        Expression::UnaryExpression(unary)
            if matches!(unary.operator, UnaryOperator::LogicalNot) =>
        {
            selector_test_span(&unary.argument)
        }
        _ => None,
    }
}
