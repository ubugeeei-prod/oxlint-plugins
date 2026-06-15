//! Rule `no-undefined-assignment` (SonarJS key S2138).
//!
//! Clean-room port from observed ESLint plugin behaviour only. The rule
//! reports a bare `undefined` identifier used as an assigned value in:
//!
//! - variable declarator initializers (`let x = undefined`)
//! - assignment expression right-hand sides (`x = undefined`, `x += undefined`)
//! - object literal property values (`{ x: undefined }`, `{ undefined }`)
//!
//! It intentionally does not flag equivalent expressions such as `void 0`,
//! default parameter/destructuring values, call arguments, array elements, or
//! class field initializers. Parentheses around the identifier are ignored.

use oxc_ast::ast::{AssignmentExpression, Expression, ObjectProperty, VariableDeclarator};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-undefined-assignment";

fn undefined_identifier_span(expr: &Expression<'_>) -> Option<Span> {
    let Expression::Identifier(ident) = expr.get_inner_expression() else {
        return None;
    };
    (ident.name == "undefined").then_some(ident.span)
}

impl Scanner<'_> {
    pub(crate) fn check_no_undefined_assignment_declarator(&mut self, it: &VariableDeclarator<'_>) {
        let Some(init) = &it.init else {
            return;
        };
        self.report_no_undefined_assignment(init);
    }

    pub(crate) fn check_no_undefined_assignment_expression(
        &mut self,
        it: &AssignmentExpression<'_>,
    ) {
        self.report_no_undefined_assignment(&it.right);
    }

    pub(crate) fn check_no_undefined_assignment_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        if it.method {
            return;
        }
        self.report_no_undefined_assignment(&it.value);
    }

    fn report_no_undefined_assignment(&mut self, expr: &Expression<'_>) {
        if let Some(span) = undefined_identifier_span(expr) {
            self.report(RULE_NAME, "useNull", span);
        }
    }
}
