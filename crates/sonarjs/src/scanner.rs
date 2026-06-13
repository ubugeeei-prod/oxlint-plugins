//! Top-level scanner state, `report*` helpers, and AST traversal for the
//! sonarjs port. Traversal uses the Oxc visitor so every node is reached; each
//! `check_*` rule body lives under [`crate::rules`].

use oxc_ast::ast::{SwitchStatement, TemplateLiteral};
use oxc_ast_visit::{Visit, walk};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::{Diagnostic, DiagnosticData, DiagnosticFix, LineIndex, SonarjsOptions};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) options: &'a SonarjsOptions,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 32]>,
    /// Number of template literals currently open on the traversal stack.
    pub(crate) template_literal_depth: u32,
    /// Number of switch statements currently open on the traversal stack.
    pub(crate) switch_depth: u32,
}

impl Scanner<'_> {
    pub(crate) fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span, None);
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
}

impl<'a> Visit<'a> for Scanner<'a> {
    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.check_no_nested_template_literals(it);
        self.template_literal_depth += 1;
        walk::walk_template_literal(self, it);
        self.template_literal_depth -= 1;
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.check_no_nested_switch(it);
        self.switch_depth += 1;
        walk::walk_switch_statement(self, it);
        self.switch_depth -= 1;
    }
}
