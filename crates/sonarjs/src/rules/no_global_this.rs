//! Rule `no-global-this` (SonarJS key S2990).
//!
//! Clean-room port. Reports any `ThisExpression` that refers to the global
//! `this` — i.e. one that is NOT inside a scope that rebinds `this`.
//!
//! Scopes that rebind `this` (so `this` is NOT flagged):
//!   - Regular (non-arrow) functions: function declarations, function
//!     expressions, object/class methods, constructors, getters, setters.
//!   - Class field/property initializers (`class C { x = this.y }`).
//!   - Class static blocks (`static { this.z() }`).
//!   - Class accessor property initializers.
//!
//! Arrow functions do NOT rebind `this`, so a `this` inside a top-level arrow
//! is still the global `this` and is therefore flagged.
//!
//! Behaviour reproduced from the public RSPEC S2990 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-global-this";

impl Scanner<'_> {
    /// Enters a `this`-rebinding scope (regular function, class field, or
    /// class static block). Increments the depth counter.
    pub(crate) fn enter_this_binding_scope(&mut self) {
        self.this_binding_depth += 1;
    }

    /// Leaves a `this`-rebinding scope. Decrements the depth counter.
    pub(crate) fn leave_this_binding_scope(&mut self) {
        self.this_binding_depth -= 1;
    }

    /// Reports a `this` expression at `span` if the binding depth is zero,
    /// meaning the expression refers to the global `this`.
    pub(crate) fn check_global_this(&mut self, span: Span) {
        if self.this_binding_depth == 0 {
            self.report(RULE_NAME, "noGlobalThis", span);
        }
    }
}
