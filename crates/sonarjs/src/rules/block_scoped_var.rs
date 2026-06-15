//! Rule `block-scoped-var` (SonarJS key S2392).
//!
//! Clean-room port. Flags `var` declarations whose binding is used outside the
//! nearest enclosing block statement — i.e., code that relies on `var`'s
//! function-level hoisting through a control-flow block boundary.
//!
//! Resolution relies on semantic analysis: the symbol's resolved references are
//! inspected and the declaration's enclosing block is located by walking the AST
//! parent chain. When semantic information is unavailable, nothing is reported
//! (conservative — no false positives).
//!
//! Behaviour is reproduced from the public SonarSource rule documentation
//! (S2392) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! ## Flagged
//! - `if (c) { var x = 1; } use(x);` — x escapes its if-block
//! - `for (var i = 0; i < n; i++) {} use(i);` — i escapes its for-loop
//! - `{ var y = 1; } use(y);` — y escapes a bare block
//!
//! ## Not flagged
//! - `function f() { var x = 1; return x; }` — x stays within the function body
//! - `if (c) { var x = 1; use(x); }` — x is used only inside the block
//! - `{ let y = 1; use(y); }` — let is never flagged
//! - `var z = 1; use(z);` — z is at program top level

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, VariableDeclarationKind, VariableDeclarator};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "block-scoped-var";

impl<'a> Scanner<'a> {
    /// Checks a `VariableDeclarator` and reports when the binding is declared
    /// with `var` inside a block and any value-reference to it lies outside that
    /// block's span.
    pub(crate) fn check_block_scoped_var(&mut self, it: &VariableDeclarator<'a>) {
        if it.kind != VariableDeclarationKind::Var {
            return;
        }

        // Only handle simple `var x` bindings; skip destructuring patterns
        // (conservative: avoids false positives in complex patterns).
        let BindingPattern::BindingIdentifier(ident) = &it.id else {
            return;
        };

        let Some(symbol_id) = ident.symbol_id.get() else {
            return;
        };

        let Some(scoping) = self.scoping else {
            return;
        };
        let Some(nodes) = self.nodes else {
            return;
        };

        let decl_node_id = scoping.symbol_declaration(symbol_id);

        let Some(block_span) = enclosing_block_span(nodes, decl_node_id) else {
            return;
        };

        // A var "escapes" when at least one value reference lies outside the
        // enclosing block's span.
        let escapes = scoping.get_resolved_references(symbol_id).any(|reference| {
            // Type-only references (e.g. `typeof x` in a type position) do not
            // constitute a runtime use outside the block.
            if !reference.is_value() {
                return false;
            }
            let AstKind::IdentifierReference(ident_ref) =
                nodes.get_node(reference.node_id()).kind()
            else {
                return false;
            };
            let ref_span = ident_ref.span;
            ref_span.start < block_span.start || ref_span.end > block_span.end
        });

        if escapes {
            self.report(RULE_NAME, "blockScopedVar", it.span);
        }
    }
}

/// Walks the AST parent chain upward from `start_id` (the `NodeId` of a
/// `VariableDeclarator`) searching for the nearest enclosing block boundary.
///
/// Returns `Some(span)` for the span of the closest block-like statement found
/// before a function body, static block, or program root. Returns `None` when
/// the `var` is at the top level of a function or the program and therefore
/// naturally visible to all code in that scope.
fn enclosing_block_span<'a>(nodes: &AstNodes<'a>, start_id: NodeId) -> Option<Span> {
    // Step past the direct parent (VariableDeclaration).
    let mut id = nodes.parent_id(start_id);
    loop {
        id = nodes.parent_id(id);
        let kind = nodes.get_node(id).kind();
        match kind {
            // Function/program-scope boundaries: var here is not block-scoped.
            AstKind::FunctionBody(_) | AstKind::Program(_) | AstKind::StaticBlock(_) => {
                return None;
            }
            // Block-like statements: var declared inside one of these escapes
            // whenever a reference lies outside the statement's span.
            AstKind::BlockStatement(b) => return Some(b.span),
            AstKind::ForStatement(f) => return Some(f.span),
            AstKind::ForInStatement(f) => return Some(f.span),
            AstKind::ForOfStatement(f) => return Some(f.span),
            AstKind::WhileStatement(w) => return Some(w.span),
            AstKind::DoWhileStatement(d) => return Some(d.span),
            AstKind::SwitchStatement(s) => return Some(s.span),
            AstKind::IfStatement(i) => return Some(i.span),
            // Intermediate nodes (e.g. SwitchCase, LabeledStatement): keep
            // walking up until a definitive boundary is found.
            _ => {}
        }
    }
}
