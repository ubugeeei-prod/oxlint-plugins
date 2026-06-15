//! Rule `no-parameter-reassignment` (SonarJS key S1226).
//!
//! Clean-room port. SonarJS S1226 — "Function parameters, caught exceptions and
//! foreach variables' initial values should not be ignored" — reports when the
//! binding introduced by a function parameter, a `catch` clause parameter, or a
//! `for-in`/`for-of` loop variable is reassigned. Overwriting such a binding
//! discards the value the runtime supplied and is a common source of confusing
//! bugs; a fresh local variable should be introduced instead.
//!
//! Only reassignment of the *binding itself* is flagged: a plain assignment
//! (`p = x`), a compound assignment (`p += 1`), or an increment/decrement
//! (`p++`, `--p`). Writing to a *property* of the binding (`p.x = 1`,
//! `p[0] = 1`) leaves the binding intact and is never flagged — those targets
//! are member expressions, not the bare identifier.
//!
//! Resolution relies on semantic analysis: an identifier write is flagged only
//! when its reference provably resolves to a parameter / catch / foreach symbol.
//! When semantic information is unavailable or the reference cannot be resolved,
//! nothing is reported (conservative — no false positives).
//!
//! Behaviour is reproduced from the public SonarSource rule documentation only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `function f(p) { p = 1; }` — reassigns a parameter
//! - `function f(p) { p++; }` — increments a parameter
//! - `const g = (a) => { a += 2; };` — compound-assigns an arrow parameter
//! - `try {} catch (e) { e = err; }` — reassigns a caught exception
//! - `for (const x of xs) { x = 0; }` — reassigns a foreach variable
//!
//! ## Not flagged
//! - `function f(p) { p.x = 1; }` — property write, binding untouched
//! - `function f(p) { const q = p; q = 2; }` — `q` is a local, not the parameter
//! - `let x = 1; x = 2;` — module-scope variable, not a parameter
//! - `for (let i = 0; i < n; i++) {}` — classic for-loop counter, not a foreach var

use oxc_ast::AstKind;
use oxc_ast::ast::IdentifierReference;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-parameter-reassignment";

impl<'a> Scanner<'a> {
    /// Returns `true` when `ident` provably resolves to a function parameter, a
    /// `catch` clause parameter, or a `for-in`/`for-of` loop variable.
    ///
    /// Returns `false` whenever semantic analysis is absent or the reference,
    /// symbol, or declaration site cannot be resolved, so callers never report
    /// on an ambiguous binding.
    fn reference_is_parameter(&self, ident: &IdentifierReference<'a>) -> bool {
        let Some(scoping) = self.scoping else {
            return false;
        };
        let Some(nodes) = self.nodes else {
            return false;
        };
        let Some(reference_id) = ident.reference_id.get() else {
            return false;
        };
        let Some(symbol_id) = scoping.get_reference(reference_id).symbol_id() else {
            return false;
        };
        let decl = scoping.symbol_declaration(symbol_id);
        match nodes.get_node(decl).kind() {
            AstKind::FormalParameter(_) | AstKind::CatchParameter(_) => true,
            // A `for-in`/`for-of` loop variable is declared by a
            // `VariableDeclarator` whose `VariableDeclaration` parent is the
            // `left` of the loop. A classic for-loop counter or an ordinary
            // declaration has a different grandparent and is not a foreach var.
            AstKind::VariableDeclarator(_) => {
                let grandparent = nodes.parent_kind(nodes.parent_id(decl));
                matches!(
                    grandparent,
                    AstKind::ForInStatement(_) | AstKind::ForOfStatement(_)
                )
            }
            _ => false,
        }
    }

    /// Reports a bare-identifier assignment target (`p = ...`, `p += 1`) when it
    /// resolves to a parameter / catch / foreach binding.
    pub(crate) fn check_no_parameter_reassignment_assignment(
        &mut self,
        ident: &IdentifierReference<'a>,
        span: oxc_span::Span,
    ) {
        if !self.reference_is_parameter(ident) {
            return;
        }
        self.report(RULE_NAME, "noParameterReassignment", span);
    }

    /// Reports an increment/decrement (`p++`, `--p`) whose target resolves to a
    /// parameter / catch / foreach binding.
    pub(crate) fn check_no_parameter_reassignment_update(
        &mut self,
        ident: &IdentifierReference<'a>,
        span: oxc_span::Span,
    ) {
        if !self.reference_is_parameter(ident) {
            return;
        }
        self.report(RULE_NAME, "noParameterReassignment", span);
    }
}
