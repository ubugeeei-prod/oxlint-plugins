//! Core scanner state for the regexp port. AST traversal lives in
//! `traversal.rs`; regexp-specific checks live in `checks.rs`.

use oxc_ast::ast::Expression;
use oxc_semantic::Scoping;
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
}
