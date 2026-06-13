//! Top-level AST scanner for the react-refresh port. Export handling lives in
//! `exports.rs` and React-component recognition in `components.rs`.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::*;
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::{
    Diagnostic, LineIndex, OnlyExportComponentsOptions, ScanState, is_react_component_name,
};

pub(crate) struct Scanner<'a> {
    pub(crate) line_index: &'a LineIndex,
    pub(crate) options: &'a OnlyExportComponentsOptions,
    pub(crate) source_text: &'a str,
    pub(crate) state: ScanState,
}

impl Scanner<'_> {
    pub(crate) fn scan_program(&mut self, body: &[Statement<'_>]) {
        for node in body {
            match node {
                Statement::ExportAllDeclaration(declaration) => {
                    if declaration.export_kind == ImportOrExportKind::Type {
                        continue;
                    }
                    self.state.has_exports = true;
                    self.report("exportAll", declaration.span);
                }
                Statement::ExportDefaultDeclaration(declaration) => {
                    self.state.has_exports = true;
                    self.handle_default_export_declaration(declaration);
                }
                Statement::ExportNamedDeclaration(declaration) => {
                    self.handle_named_export_declaration(declaration);
                }
                Statement::VariableDeclaration(declaration) => {
                    self.collect_local_variable_components(declaration);
                }
                Statement::FunctionDeclaration(function) => {
                    if let Some(id) = &function.id
                        && is_react_component_name(id.name.as_str())
                    {
                        self.state.local_components.push(id.span);
                    }
                }
                Statement::ImportDeclaration(declaration)
                    if declaration.source.value.as_str() == "react" =>
                {
                    self.state.react_is_in_scope = true;
                }
                _ => {}
            }
        }
    }

    pub(crate) fn finish(mut self) -> SmallVec<[Diagnostic; 8]> {
        if self.options.check_js && !self.state.react_is_in_scope {
            return SmallVec::new();
        }

        if self.state.has_exports {
            if self.state.has_react_export {
                for span in std::mem::take(&mut self.state.non_component_exports) {
                    self.report("namedExport", span);
                }
                for span in std::mem::take(&mut self.state.react_context_exports) {
                    self.report("reactContext", span);
                }
            } else if !self.state.local_components.is_empty() {
                for span in std::mem::take(&mut self.state.local_components) {
                    self.report("localComponents", span);
                }
            }
        } else if !self.state.local_components.is_empty() {
            for span in std::mem::take(&mut self.state.local_components) {
                self.report("noExport", span);
            }
        }

        self.state.diagnostics
    }

    pub(crate) fn report(&mut self, message_id: &'static str, span: Span) {
        self.state.diagnostics.push(Diagnostic {
            message_id,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}
