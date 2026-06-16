//! Rule `no-unused-vars` (SonarJS key S1481).
//!
//! Clean-room port. Behavior derived solely from the public RSPEC for S1481
//! ("Unused local variables and functions should be removed"): a locally
//! declared variable that is never read is dead code and should be removed.
//! The authoritative RSPEC Noncompliant example is:
//!
//! ```js
//! function numberOfMinutes(hours) {
//!   var seconds = 0;   // Noncompliant: 'seconds' is never used
//!   return hours * 60;
//! }
//! ```
//!
//! ## Requires semantic analysis
//!
//! Reference resolution is delegated to `oxc_semantic`. The rule reads the
//! binding's `symbol_id` and asks `scoping.symbol_is_unused(symbol_id)`, which
//! correctly accounts for reads in nested scopes and closures. When semantic
//! data is absent nothing is emitted.
//!
//! ## Conservative zero-false-positive subset
//!
//! This port deliberately under-reports to guarantee no false positives:
//!
//! - Only PLAIN identifier bindings (`var x` / `let x` / `const x`) are
//!   considered. Destructuring patterns (`ObjectPattern` / `ArrayPattern`) are
//!   skipped entirely — tracking individual destructured bindings is more
//!   involved and risks false positives.
//! - Names beginning with `_` are treated as intentional unused placeholders
//!   and are exempt (conventional marker).
//! - ONLY function-local (i.e. non-top-level) variables are flagged. A binding
//!   whose declaring scope is the root/top-level (module or script) scope is
//!   never flagged, because such a declaration may be exported or otherwise
//!   form part of the program's public API. The scope gate compares
//!   `scoping.symbol_scope_id(symbol_id)` against `scoping.root_scope_id()`.
//! - Unused functions (another facet of S1481) are out of scope here; only
//!   variable declarators are inspected, so unused functions are under-reported.
//!
//! ## Flagged
//! - `function f() { var seconds = 0; return 60; }` — `seconds` never read,
//!   declared in the function scope → flagged
//! - `function f() { let x = 1; }` — unused function-local binding → flagged
//!
//! ## Not flagged
//! - `function f() { var x = 1; return x; }` — `x` is read
//! - `function f() { var _ignored = 1; return 60; }` — underscore-prefixed
//! - `const topLevelUnused = 1;` — top-level/exportable declaration (scope gate)
//! - `function f() { const { a } = obj; return 60; }` — destructuring skipped
//! - `function f() { var x = 1; return () => x; }` — read inside a closure;
//!   semantic resolves the reference regardless of nesting

use oxc_ast::ast::{BindingPattern, VariableDeclarator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-unused-vars";

impl<'a> Scanner<'a> {
    /// Flag a function-local variable declared with a plain identifier binding
    /// that is never read. See the module docs for the conservative subset.
    pub(crate) fn check_no_unused_vars(&mut self, it: &VariableDeclarator<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        // Reference resolution requires semantic data; bail out without it.
        let Some(scoping) = self.scoping else {
            return;
        };
        // Only handle plain identifier bindings; destructuring is out of scope.
        let BindingPattern::BindingIdentifier(binding) = &it.id else {
            return;
        };
        // Underscore-prefixed names are intentional unused placeholders.
        if binding.name.starts_with('_') {
            return;
        }
        let Some(symbol_id) = binding.symbol_id.get() else {
            return;
        };
        // Zero-FP scope gate: never flag top-level (module/script) declarations,
        // which may be exported or part of the program's public API. Only
        // variables in a nested (function/block) scope are eligible.
        if scoping.symbol_scope_id(symbol_id) == scoping.root_scope_id() {
            return;
        }
        if scoping.symbol_is_unused(symbol_id) {
            self.report(RULE_NAME, "unusedVariable", binding.span);
        }
    }
}
