//! Core scanner state for the regexp port. AST traversal lives in
//! `traversal.rs`; regexp-specific checks live in `checks.rs`.

use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::types::{Diagnostic, DiagnosticData, LineIndex};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn report(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
    ) {
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
}
