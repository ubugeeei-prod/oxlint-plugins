//! Core scanner state for the regexp port. AST traversal lives in
//! `traversal.rs`; regexp-specific checks live in `checks.rs`.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::types::{Diagnostic, DiagnosticData, LineIndex};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    /// Scoping information from semantic analysis. Used for reference
    /// resolution (e.g. to detect whether a `RegExp` identifier is the
    /// global constructor or a shadowed local binding).
    pub(crate) scoping: &'a Scoping,
    /// AST nodes from semantic analysis. Used for declaration-site lookup
    /// (e.g. to check whether a variable is initialised with a string
    /// literal).
    pub(crate) nodes: &'a AstNodes<'a>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span);
    }

    pub(crate) fn report_with_data(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: DiagnosticData,
        span: Span,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }

    /// Returns `true` when `callee` is the global `RegExp` constructor (an
    /// unqualified `RegExp` identifier that is not shadowed by a local
    /// binding). A shadowed `RegExp` (e.g. a function parameter) resolves to
    /// a local symbol and should not be treated as the global constructor.
    ///
    /// The check relies on the `reference_id` that the semantic analyser
    /// attaches to every [`IdentifierReference`]: if the reference resolves to
    /// a local symbol, `symbol_id()` will be `Some(…)`; if it is unresolved
    /// (i.e. a free/global reference), `symbol_id()` will be `None`.
    pub(crate) fn is_global_regexp_callee(&self, callee: &Expression) -> bool {
        let Expression::Identifier(ident) = callee else {
            return false;
        };
        if ident.name != "RegExp" {
            return false;
        }
        // After semantic analysis the `reference_id` cell is always `Some`.
        // If for any reason it is still `None` (e.g. in a test that bypasses
        // semantic), treat the identifier conservatively as global so existing
        // behaviour is preserved.
        let Some(reference_id) = ident.reference_id.get() else {
            return true;
        };
        // A resolved reference has a `symbol_id`; an unresolved (global)
        // reference does not.
        self.scoping
            .get_reference(reference_id)
            .symbol_id()
            .is_none()
    }

    /// Returns `true` when `expr` is statically known to be a `string` value.
    ///
    /// Recognised forms:
    /// - A string literal (`"foo"`, `'bar'`).
    /// - A no-expression template literal (`` `foo` ``).
    /// - An identifier that resolves to a `const`/`let`/`var` binding whose
    ///   initialiser is itself a known-string expression (one level of
    ///   indirection only — we do not follow chains of aliases).
    ///
    /// Everything else (free/global identifiers, call results, member access,
    /// etc.) is conservatively treated as *not* a known string so that we do
    /// not produce false positives.
    pub(crate) fn receiver_is_known_string(&self, expr: &Expression) -> bool {
        match expr.get_inner_expression() {
            Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(tmpl) => tmpl.expressions.is_empty(),
            Expression::Identifier(ident) => {
                // Resolve the reference to its declaration symbol.
                let Some(reference_id) = ident.reference_id.get() else {
                    // No reference id means semantic did not run; be conservative.
                    return false;
                };
                let Some(symbol_id) = self.scoping.get_reference(reference_id).symbol_id() else {
                    // symbol_id() is None → free/unresolved (global) reference.
                    return false;
                };
                // Look up the declaration AST node for this symbol.
                let decl_node_id = self.scoping.symbol_declaration(symbol_id);
                let decl_kind = self.nodes.get_node(decl_node_id).kind();
                let AstKind::VariableDeclarator(declarator) = decl_kind else {
                    return false;
                };
                // The initialiser must itself be a known string.
                declarator.init.as_ref().is_some_and(|init| {
                    matches!(init.get_inner_expression(), Expression::StringLiteral(_))
                        || matches!(init.get_inner_expression(),
                            Expression::TemplateLiteral(t) if t.expressions.is_empty()
                        )
                })
            }
            _ => false,
        }
    }
}
