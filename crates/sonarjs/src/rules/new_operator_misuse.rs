//! Rule `new-operator-misuse` (SonarJS key S2999).
//!
//! Clean-room port. Arrow functions cannot be used as constructors; calling
//! `new` on an arrow function always throws a `TypeError` at runtime. This
//! rule flags `NewExpression` nodes whose callee is an arrow function, either
//! directly (an inline arrow literal as callee, e.g. `new (() => {})()`) or
//! indirectly via a never-reassigned `const`/`let`/`var` binding whose
//! initializer is an arrow function (e.g. `const f = () => {}; new f()`).
//!
//! **Narrowing (conservative approach)**:
//! Only arrow functions are flagged. Regular `function` declarations/expressions
//! and classes are valid constructors and are NOT flagged. Member expressions
//! (`new obj.method()`), unresolved identifiers (`new Foo()`), and identifiers
//! whose declaration cannot be statically resolved are all left alone to avoid
//! false positives. This means the rule under-reports relative to a full
//! type-aware analysis, which is an acceptable trade-off.
//!
//! Behaviour is reproduced from the public RSPEC description (S2999) and
//! observed language semantics only. No upstream source, tests, fixtures, helper
//! code, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `new (() => {})()` — direct arrow-function callee
//! - `const f = () => {}; new f();` — identifier resolving to arrow
//!
//! ## Not flagged
//! - `new Foo()` — unresolved identifier (no semantic proof it is an arrow)
//! - `function F() {} new F();` — regular function, valid constructor
//! - `class C {} new C();` — class, valid constructor
//! - `new obj.method()` — member expression, cannot statically resolve
//! - `const g = function() {}; new g();` — regular function expression

use oxc_ast::ast::{Expression, NewExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "new-operator-misuse";

impl<'a> Scanner<'a> {
    /// Reports a `NewExpression` whose callee is an arrow function (directly or
    /// via identifier resolution). Arrow functions are not constructors and
    /// always throw a `TypeError`.
    pub(crate) fn check_new_operator_misuse(&mut self, expr: &NewExpression<'a>) {
        let callee = expr.callee.get_inner_expression();
        let is_arrow = match callee {
            Expression::ArrowFunctionExpression(_) => true,
            Expression::Identifier(ident) => match self.resolve_identifier_initializer(ident) {
                Some(init) => {
                    matches!(
                        init.get_inner_expression(),
                        Expression::ArrowFunctionExpression(_)
                    )
                }
                None => false,
            },
            _ => false,
        };
        if is_arrow {
            self.report(RULE_NAME, "newOperatorMisuse", expr.span);
        }
    }
}
