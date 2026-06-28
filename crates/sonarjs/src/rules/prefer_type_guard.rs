//! Rule `prefer-type-guard` (SonarJS key S4322).
//!
//! Clean-room port. TypeScript lets a function narrow its argument's type for
//! callers by declaring a *type predicate* return type (`arg is SomeType`)
//! instead of a plain `boolean`. A function whose entire job is a single
//! `instanceof` test on its parameter is the textbook case for a type guard:
//! declaring `arg is SomeType` lets the compiler narrow the value at every call
//! site, whereas returning `boolean` throws that information away.
//!
//! ```ts
//! function isCat(animal: Animal): boolean {   // Noncompliant
//!   return animal instanceof Cat;
//! }
//!
//! function isCat(animal: Animal): animal is Cat {   // Compliant
//!   return animal instanceof Cat;
//! }
//! ```
//!
//! ## Narrow form
//!
//! To stay strictly false-positive-free without a type checker, this port only
//! flags the unambiguous shape:
//!
//! * exactly one parameter, and that parameter is a plain binding identifier
//!   (no destructuring, no rest, no extra parameters);
//! * the body is a *single* `return <expr>;` (or, for an arrow, a single
//!   expression body), where `<expr>` (parentheses stripped) is
//!   `<param> instanceof <Constructor>` — the left operand being an identifier
//!   that names that one parameter;
//! * the function does NOT already declare a type-predicate return type.
//!
//! Functions that already return `arg is T`, that mix the check with other
//! statements, that combine several conditions (`x instanceof A || ...`), or
//! that negate the test (`!(x instanceof A)`) are intentionally left alone:
//! converting those to a type guard is not a mechanical, always-correct rewrite,
//! so they are a documented follow-up. Both concrete functions/methods and
//! arrow functions (block- or expression-bodied) are covered.
//!
//! Behaviour is reproduced from the public RSPEC description (S4322) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, Expression, FormalParameters, Function, Statement,
    TSType, TSTypeAnnotation,
};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-type-guard";

impl<'a> Scanner<'a> {
    /// Entry for concrete functions, function expressions and methods. Bodiless
    /// nodes (overload/abstract/ambient signatures) have no statements and are
    /// skipped automatically.
    pub(crate) fn check_prefer_type_guard(&mut self, func: &Function<'a>) {
        if return_type_is_predicate(func.return_type.as_deref()) {
            return;
        }
        let Some(body) = &func.body else {
            return;
        };
        let Some(param) = single_param_name(&func.params) else {
            return;
        };
        let Some(expr) = single_return_expr(&body.statements) else {
            return;
        };
        if is_instanceof_on_param(skip_parens(expr), param) {
            let span = func.id.as_ref().map_or(func.span, |id| id.span);
            self.report(RULE_NAME, "preferTypeGuard", span);
        }
    }

    /// Entry for arrow functions, covering both block bodies (single `return`)
    /// and expression bodies (`p => p instanceof T`).
    pub(crate) fn check_prefer_type_guard_arrow(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        if return_type_is_predicate(arrow.return_type.as_deref()) {
            return;
        }
        let Some(param) = single_param_name(&arrow.params) else {
            return;
        };
        let expr = if arrow.expression {
            // Expression-bodied arrows wrap the expression in a single
            // `ExpressionStatement` inside the synthetic function body.
            let Some(Statement::ExpressionStatement(stmt)) = arrow.body.statements.first() else {
                return;
            };
            &stmt.expression
        } else {
            let Some(expr) = single_return_expr(&arrow.body.statements) else {
                return;
            };
            expr
        };
        if is_instanceof_on_param(skip_parens(expr), param) {
            self.report(RULE_NAME, "preferTypeGuard", arrow.span);
        }
    }
}

/// `true` when the return type annotation is already a type predicate
/// (`arg is T`), in which case the function is a type guard and must be left
/// alone.
fn return_type_is_predicate(annotation: Option<&TSTypeAnnotation<'_>>) -> bool {
    matches!(
        annotation,
        Some(annotation) if matches!(annotation.type_annotation, TSType::TSTypePredicate(_))
    )
}

/// Returns the name of the sole parameter when the parameter list is exactly one
/// plain binding identifier (no rest, no destructuring, no extra parameters).
fn single_param_name<'a>(params: &FormalParameters<'a>) -> Option<&'a str> {
    if params.rest.is_some() || params.items.len() != 1 {
        return None;
    }
    let BindingPattern::BindingIdentifier(id) = &params.items[0].pattern else {
        return None;
    };
    Some(id.name.as_str())
}

/// Returns the argument of a lone `return <expr>;` statement, or `None` when the
/// body is not exactly one value-returning `return`.
fn single_return_expr<'b, 'a>(statements: &'b [Statement<'a>]) -> Option<&'b Expression<'a>> {
    if statements.len() != 1 {
        return None;
    }
    let Statement::ReturnStatement(ret) = &statements[0] else {
        return None;
    };
    ret.argument.as_ref()
}

/// Strips redundant parentheses so `(x instanceof T)` is treated like
/// `x instanceof T`.
fn skip_parens<'b, 'a>(expr: &'b Expression<'a>) -> &'b Expression<'a> {
    let mut current = expr;
    while let Expression::ParenthesizedExpression(paren) = current {
        current = &paren.expression;
    }
    current
}

/// `true` when `expr` is `<param> instanceof <anything>`, i.e. an `instanceof`
/// binary expression whose left operand is an identifier naming `param`.
fn is_instanceof_on_param(expr: &Expression<'_>, param: &str) -> bool {
    let Expression::BinaryExpression(binary) = expr else {
        return false;
    };
    if binary.operator != BinaryOperator::Instanceof {
        return false;
    }
    let Expression::Identifier(ident) = skip_parens(&binary.left) else {
        return false;
    };
    ident.name.as_str() == param
}
