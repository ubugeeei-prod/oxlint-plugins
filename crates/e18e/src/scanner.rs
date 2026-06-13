//! Top-level scanner state and `report*` helpers for the e18e port. Statement
//! and expression traversal live in `statements.rs` and `expressions.rs`; each
//! `check_*` rule body lives under [`crate::rules`].

use oxc_ast::ast::Program;
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{Diagnostic, DiagnosticData, DiagnosticFix, E18eOptions, LineIndex};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) options: &'a E18eOptions,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 32]>,
    pub(crate) function_depth: usize,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan_program(&mut self, program: &'a Program<'a>) {
        for statement in &program.body {
            self.scan_statement(statement);
        }
    }

    pub(crate) fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span, None);
    }

    pub(crate) fn report_with_fix(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        replacement: impl Into<CompactString>,
    ) {
        self.report_with_data(
            rule_name,
            message_id,
            DiagnosticData::default(),
            span,
            Some(DiagnosticFix {
                start: span.start,
                end: span.end,
                replacement: replacement.into(),
            }),
        );
    }

    pub(crate) fn report_with_data(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: DiagnosticData,
        span: Span,
        fix: Option<DiagnosticFix>,
    ) {
        if !self.options.has_rule(rule_name) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix,
        });
    }

    pub(crate) fn text(&self, span: Span) -> &'a str {
        &self.source_text[span.start as usize..span.end as usize]
    }
}
