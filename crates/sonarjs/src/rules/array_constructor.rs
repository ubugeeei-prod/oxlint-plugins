//! Rule `array-constructor` (SonarJS key S1528).
//!
//! Clean-room port. The `Array` constructor is an error-prone way to build an
//! array: `Array(1, 2, 3)` produces `[1, 2, 3]`, but `Array(3)` produces a
//! sparse array of length 3 with no elements. The single-argument form is the
//! only one whose meaning is unambiguous, so any other arity should use an
//! array literal (`[]`) instead.
//!
//! **Flagged** — a call or `new` of the bare `Array` identifier with an
//! argument count other than one, and no TypeScript type arguments:
//! - `Array(1, 2, 3)` — multiple arguments.
//! - `new Array(1, 2, 3)` — multiple arguments.
//! - `Array()` / `new Array()` — zero arguments (use `[]`).
//!
//! **Not flagged**:
//! - `Array(500)` / `new Array(len)` — a single argument is the (unambiguous)
//!   array length.
//! - `Array<number>(1, 2, 3)` — explicit type arguments signal an intentional
//!   typed construction.
//! - `foo.Array(1, 2)` — the callee is not the bare `Array` identifier.
//!
//! Behaviour is reproduced from the public RSPEC description (S1528) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression, NewExpression};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "array-constructor";

impl Scanner<'_> {
    pub(crate) fn check_array_constructor_call(&mut self, expr: &CallExpression<'_>) {
        self.check_array_constructor(
            &expr.callee,
            expr.type_arguments.is_some(),
            expr.arguments.len(),
            expr.span,
        );
    }

    pub(crate) fn check_array_constructor_new(&mut self, expr: &NewExpression<'_>) {
        self.check_array_constructor(
            &expr.callee,
            expr.type_arguments.is_some(),
            expr.arguments.len(),
            expr.span,
        );
    }

    fn check_array_constructor(
        &mut self,
        callee: &Expression<'_>,
        has_type_arguments: bool,
        argument_count: usize,
        span: Span,
    ) {
        if has_type_arguments || argument_count == 1 {
            return;
        }
        let Expression::Identifier(identifier) = callee.get_inner_expression() else {
            return;
        };
        if identifier.name.as_str() != "Array" {
            return;
        }
        self.report(RULE_NAME, "arrayConstructor", span);
    }
}
