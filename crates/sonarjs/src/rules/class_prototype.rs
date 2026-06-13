//! Rule `class-prototype` (SonarJS key S3525).
//!
//! Clean-room port. Assigning methods or properties directly to a constructor's
//! `.prototype` is the old-style way to define class behaviour. Modern JavaScript
//! provides `class` syntax that is easier to read, refactor, and tool-analyse.
//!
//! ## Flagged forms
//!
//! Any `AssignmentExpression` whose left-hand side is a static member expression
//! of the form `<X>.prototype.<member>`. Examples:
//!
//! ```js
//! Foo.prototype.bar = function () {};   // flagged
//! Foo.prototype.baz = 1;               // flagged
//! a.b.prototype.c = x;                 // flagged (outer.object is a.b.prototype)
//! ```
//!
//! ## Not flagged
//!
//! - `Foo.prototype = {}` — the LHS property IS `prototype` itself; there is no
//!   `.member` after it. The outer member's `object` is plain `Foo`, not a
//!   `.prototype` expression.
//! - `foo.bar = 1` — no `.prototype` in the chain.
//! - `Foo.prototype` read without assignment.
//! - Computed access: `Foo.prototype["bar"] = x` — out of scope for this PR
//!   (only the static-member `.prototype.name` form is covered; computed-member
//!   support can be added as a follow-up).
//!
//! ## Detection strategy
//!
//! `visit_assignment_expression` checks whether `assign.left` is
//! `AssignmentTarget::StaticMemberExpression(outer)` and whether
//! `outer.object.get_inner_expression()` resolves to a
//! `Expression::StaticMemberExpression(inner)` whose `inner.property.name` is
//! `"prototype"`. Reports the full assignment expression's span.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{AssignmentExpression, AssignmentTarget, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "class-prototype";

impl Scanner<'_> {
    pub(crate) fn check_class_prototype(&mut self, assign: &AssignmentExpression<'_>) {
        let AssignmentTarget::StaticMemberExpression(outer) = &assign.left else {
            return;
        };
        let Expression::StaticMemberExpression(inner) = outer.object.get_inner_expression() else {
            return;
        };
        if inner.property.name.as_str() != "prototype" {
            return;
        }
        self.report(RULE_NAME, "classPrototype", assign.span);
    }
}
