//! Scanner state, scope management, and reporting helpers for the security
//! port. Traversal lives in `statements.rs` and `expressions.rs`; per-rule
//! diagnostics in `checks.rs`; static-expression and import analysis in
//! `analysis.rs`.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::Statement;
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{Binding, Diagnostic, DiagnosticData, LineIndex, Scope};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    pub(crate) scopes: SmallVec<[Scope; 8]>,
    pub(crate) csrf_seen: bool,
    pub(crate) comment_spans: SmallVec<[Span; 16]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub(crate) fn pop_scope(&mut self) {
        let _ = self.scopes.pop();
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes
            .last_mut()
            .expect("scanner always has an active scope")
    }

    pub(crate) fn bind(&mut self, name: &str, binding: Binding) {
        self.current_scope_mut()
            .bindings
            .insert(CompactString::from(name), binding);
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<&Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.bindings.get(name))
    }

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

    pub(crate) fn scan_program(&mut self, body: &'a [Statement<'a>]) {
        for statement in body {
            self.scan_statement(statement);
        }
    }
}
