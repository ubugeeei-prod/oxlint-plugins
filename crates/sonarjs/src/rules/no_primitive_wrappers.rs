//! Rule `no-primitive-wrappers` (SonarJS key S1533).
//!
//! Clean-room port. Calling `new Number(...)`, `new String(...)`, or
//! `new Boolean(...)` creates a wrapper *object* rather than a primitive value.
//! This has surprising consequences: `typeof new Number(1) === 'object'`, and
//! `new Boolean(false)` is truthy because any object is truthy. The wrapper
//! object form should never be used; use the bare conversion calls (`Number(x)`,
//! `String(x)`, `Boolean(x)`) when a type conversion is needed, or just the
//! primitive literals directly.
//!
//! **Flagged**:
//! - `new Number(1)` — wraps a number in an object.
//! - `new String('x')` — wraps a string in an object.
//! - `new Boolean(false)` — wraps a boolean in an object (always truthy!).
//! - `const n = new Number(x);` — assignment doesn't change the object-ness.
//!
//! **Not flagged**:
//! - `Number(1)` — plain call (no `new`), performs a type conversion to a
//!   primitive; this is fine.
//! - `new Array(3)` — not a primitive wrapper constructor.
//! - `new Foo()` — not a primitive wrapper constructor.
//! - `String(x)` — plain conversion call.
//!
//! Behaviour is reproduced from the public RSPEC description (S1533) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, NewExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-primitive-wrappers";

const WRAPPERS: [&str; 3] = ["Number", "String", "Boolean"];

impl Scanner<'_> {
    pub(crate) fn check_no_primitive_wrappers(&mut self, expr: &NewExpression<'_>) {
        let Expression::Identifier(callee) = expr.callee.get_inner_expression() else {
            return;
        };
        if !WRAPPERS.contains(&callee.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "primitiveWrapper", expr.span);
    }
}
