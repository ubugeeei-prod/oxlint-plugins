//! Export-statement handlers for the react-refresh scanner.

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};

use crate::helpers::{
    binding_identifier, binding_pattern_identifier, identifier_reference,
    is_constant_export_expression, is_create_context_call, is_react_class_component,
    named_export_identifier,
};
use crate::scanner::Scanner;
use crate::{ComponentCheck, NamedSpan, is_react_component_name};

impl Scanner<'_> {
    pub(crate) fn handle_named_export_declaration(
        &mut self,
        declaration: &ExportNamedDeclaration<'_>,
    ) {
        if declaration.export_kind == ImportOrExportKind::Type {
            return;
        }

        if let Some(Declaration::FunctionDeclaration(function)) = &declaration.declaration
            && function.r#type == FunctionType::TSDeclareFunction
        {
            return;
        }

        self.state.has_exports = true;
        if let Some(declaration) = &declaration.declaration {
            self.handle_export_declaration(declaration);
        }

        for specifier in &declaration.specifiers {
            if specifier.export_kind == ImportOrExportKind::Type {
                continue;
            }
            let exported = named_export_identifier(&specifier.exported);
            if exported.is_some_and(|named| named.name == "default") {
                self.handle_export_name(
                    named_export_identifier(&specifier.local),
                    None,
                    specifier.local.span(),
                );
            } else {
                self.handle_export_name(exported, None, specifier.exported.span());
            }
        }
    }

    pub(crate) fn handle_default_export_declaration(
        &mut self,
        declaration: &ExportDefaultDeclaration<'_>,
    ) {
        match &declaration.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.handle_export_name(Some(binding_identifier(id)), None, id.span);
                } else {
                    self.report("anonymousExport", function.span);
                }
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    if is_react_class_component(class) {
                        self.state.has_react_export = true;
                    } else {
                        self.state.non_component_exports.push(id.span);
                    }
                } else {
                    self.report("anonymousExport", class.span);
                }
            }
            ExportDefaultDeclarationKind::CallExpression(call) => {
                self.handle_export_call_expression(call);
            }
            ExportDefaultDeclarationKind::Identifier(identifier) => {
                self.handle_export_name(
                    Some(identifier_reference(identifier)),
                    None,
                    identifier.span,
                );
            }
            ExportDefaultDeclarationKind::ArrowFunctionExpression(_) => {
                self.report("anonymousExport", declaration.span);
            }
            ExportDefaultDeclarationKind::TSAsExpression(expression) => {
                self.handle_default_export_expression(&expression.expression, declaration.span);
            }
            ExportDefaultDeclarationKind::TSSatisfiesExpression(expression) => {
                self.handle_default_export_expression(&expression.expression, declaration.span);
            }
            ExportDefaultDeclarationKind::TSTypeAssertion(expression) => {
                self.handle_default_export_expression(&expression.expression, declaration.span);
            }
            ExportDefaultDeclarationKind::TSNonNullExpression(expression) => {
                self.handle_default_export_expression(&expression.expression, declaration.span);
            }
            ExportDefaultDeclarationKind::TSInstantiationExpression(expression) => {
                self.handle_default_export_expression(&expression.expression, declaration.span);
            }
            _ => {
                self.state.non_component_exports.push(declaration.span);
            }
        }
    }

    fn handle_default_export_expression(&mut self, expression: &Expression<'_>, span: Span) {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => self.handle_export_call_expression(call),
            Expression::Identifier(identifier) => {
                self.handle_export_name(
                    Some(identifier_reference(identifier)),
                    None,
                    identifier.span,
                );
            }
            Expression::ArrowFunctionExpression(_) => self.report("anonymousExport", span),
            _ => self.state.non_component_exports.push(span),
        }
    }

    fn handle_export_call_expression(&mut self, call: &CallExpression<'_>) {
        match self.is_call_expression_react_component(call) {
            ComponentCheck::No => self.state.non_component_exports.push(call.span),
            ComponentCheck::NeedName => self.report("anonymousExport", call.span),
            ComponentCheck::Yes => self.state.has_react_export = true,
        }
    }

    fn handle_export_declaration(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                for variable in &declaration.declarations {
                    let fallback = variable.id.span();
                    let name = binding_pattern_identifier(&variable.id);
                    if let Some(init) = &variable.init {
                        self.handle_export_name(name, Some(init), fallback);
                    } else {
                        self.handle_export_name(name, None, fallback);
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.handle_export_name(Some(binding_identifier(id)), None, id.span);
                } else {
                    self.report("anonymousExport", function.span);
                }
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    if is_react_class_component(class) {
                        self.state.has_react_export = true;
                    } else {
                        self.state.non_component_exports.push(id.span);
                    }
                } else {
                    self.report("anonymousExport", class.span);
                }
            }
            _ => self.state.non_component_exports.push(declaration.span()),
        }
    }

    fn handle_export_name(
        &mut self,
        name: Option<NamedSpan<'_>>,
        init: Option<&Expression<'_>>,
        fallback: Span,
    ) {
        let Some(name) = name else {
            self.state.non_component_exports.push(fallback);
            return;
        };

        if self
            .options
            .allow_export_names
            .iter()
            .any(|allowed| allowed == name.name)
        {
            return;
        }

        let Some(init) = init.map(Expression::get_inner_expression) else {
            if is_react_component_name(name.name) {
                self.state.has_react_export = true;
            } else {
                self.state.non_component_exports.push(name.span);
            }
            return;
        };

        if self.options.allow_constant_export && is_constant_export_expression(init) {
            return;
        }

        if is_create_context_call(init) {
            self.state.react_context_exports.push(name.span);
            return;
        }

        let is_react_component = is_react_component_name(name.name)
            && self.is_expression_react_component(init) != ComponentCheck::No;

        if is_react_component {
            self.state.has_react_export = true;
        } else {
            self.state.non_component_exports.push(name.span);
        }
    }

    pub(crate) fn collect_local_variable_components(
        &mut self,
        declaration: &VariableDeclaration<'_>,
    ) {
        for variable in &declaration.declarations {
            let Some(name) = binding_pattern_identifier(&variable.id) else {
                continue;
            };
            let Some(init) = &variable.init else {
                continue;
            };
            if is_react_component_name(name.name)
                && self.is_expression_react_component(init) != ComponentCheck::No
            {
                self.state.local_components.push(name.span);
            }
        }
    }
}
