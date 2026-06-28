//! Rule `no-implicit-global` (SonarJS key S2703).
//!
//! Clean-room port. SonarJS S2703 — "Variables and functions should not be
//! declared in the global scope" — targets the *accidental* creation of global
//! variables: assigning to a name that was never declared with `var`, `let` or
//! `const`. Such a write leaks the name onto the global object, where it can be
//! read and mutated from anywhere, which is a common source of subtle bugs.
//!
//! Behaviour is reproduced from the public SonarSource rule description only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Detection (semantic, conservative)
//! An assignment target (`x = …`, `x += …`) or an increment/decrement target
//! (`x++`, `--x`) is flagged only when its identifier reference provably
//! resolves to *no* declared symbol — i.e. it is an unresolved reference that
//! escapes to the global scope. A reference that resolves to any local, block,
//! function, module, or parameter binding is never flagged, because the write
//! stays inside a real scope. When semantic information is unavailable the rule
//! reports nothing (no false positives).
//!
//! To stay false-positive-free against host environments, writes to a curated
//! set of well-known predefined globals (browser/Node/ES builtins such as
//! `window`, `location`, `name`, `module`, `globalThis`) are not reported —
//! these mirror the predefined environment globals that SonarJS itself excludes,
//! and reassigning them is an environment interaction rather than an accidental
//! implicit declaration. The rule under-reports rather than over-reports.
//!
//! ## Flagged
//! - `function f() { x = 1; }` — `x` was never declared; leaks to the global scope
//! - `function f() { for (i = 0; i < n; i++) {} }` — `i` and `i++` are implicit globals
//! - `function f() { counter++; }` — increment of an undeclared name
//! - `total = 5;` at top level where `total` is never declared
//!
//! ## Not flagged
//! - `function f() { let x; x = 1; }` — `x` resolves to a local binding
//! - `var total = 0; total = 5;` — `total` is explicitly declared
//! - `window.foo = 1; obj.bar = 2;` — property writes, not bare identifiers
//! - `location = url;` — `location` is a predefined environment global

use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, IdentifierReference, SimpleAssignmentTarget,
    UpdateExpression,
};
use oxlint_plugins_carton::CompactString;

use crate::{DiagnosticData, scanner::Scanner};

pub(crate) const RULE_NAME: &str = "no-implicit-global";

impl<'a> Scanner<'a> {
    /// Flags `x = …` / `x += …` when `x` is an undeclared (global-escaping) name.
    pub(crate) fn check_no_implicit_global(&mut self, expr: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(ident) = &expr.left {
            self.report_implicit_global(ident);
        }
    }

    /// Flags `x++` / `--x` when `x` is an undeclared (global-escaping) name.
    pub(crate) fn check_no_implicit_global_update(&mut self, expr: &UpdateExpression<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) = &expr.argument {
            self.report_implicit_global(ident);
        }
    }

    fn report_implicit_global(&mut self, ident: &IdentifierReference<'a>) {
        // Resolution requires semantic analysis; without it we report nothing.
        let Some(scoping) = self.scoping else {
            return;
        };
        let Some(reference_id) = ident.reference_id.get() else {
            return;
        };
        // A resolved reference points at a real declared binding — not implicit.
        if scoping.get_reference(reference_id).symbol_id().is_some() {
            return;
        }
        let name = ident.name.as_str();
        if is_predefined_global(name) {
            return;
        }
        let data = DiagnosticData {
            value: Some(CompactString::from(name)),
            format: None,
        };
        self.report_with_data(RULE_NAME, "implicitGlobal", data, ident.span, None);
    }
}

/// Well-known predefined globals (ES, browser, Node) whose reassignment is an
/// environment interaction rather than an accidental implicit declaration.
/// Writes to these are intentionally not flagged to avoid false positives.
fn is_predefined_global(name: &str) -> bool {
    matches!(
        name,
        // ECMAScript globals / value keywords
        "globalThis"
            | "undefined"
            | "NaN"
            | "Infinity"
            | "eval"
            // Browser globals
            | "window"
            | "self"
            | "document"
            | "location"
            | "navigator"
            | "history"
            | "screen"
            | "console"
            | "name"
            | "status"
            | "top"
            | "parent"
            | "opener"
            | "frames"
            | "length"
            | "origin"
            | "event"
            | "localStorage"
            | "sessionStorage"
            // Node globals
            | "global"
            | "process"
            | "module"
            | "exports"
            | "require"
            | "__dirname"
            | "__filename"
            | "Buffer"
    )
}
