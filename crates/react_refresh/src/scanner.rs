//! AST scanner for the react-refresh port. Contains the `Scanner` struct and
//! every traversal / rule check method as an `impl Scanner` block.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::*;
use crate::{
    is_constant_export_expression_kind, is_react_component_name, ComponentCheck, Diagnostic,
    LineIndex, NamedSpan, OnlyExportComponentsOptions, ScanState, DEFAULT_HOCS,
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

    fn handle_named_export_declaration(&mut self, declaration: &ExportNamedDeclaration<'_>) {
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

    fn handle_default_export_declaration(&mut self, declaration: &ExportDefaultDeclaration<'_>) {
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

    fn collect_local_variable_components(&mut self, declaration: &VariableDeclaration<'_>) {
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

    fn is_expression_react_component(&self, expression: &Expression<'_>) -> ComponentCheck {
        match expression.get_inner_expression() {
            Expression::Identifier(identifier) => {
                component_check_for_name(identifier.name.as_str())
            }
            Expression::ArrowFunctionExpression(function) => {
                if function_param_count(&function.params) > 2 {
                    ComponentCheck::No
                } else {
                    ComponentCheck::NeedName
                }
            }
            Expression::FunctionExpression(function) => {
                if function_param_count(&function.params) > 2 {
                    ComponentCheck::No
                } else if let Some(id) = &function.id {
                    component_check_for_name(id.name.as_str())
                } else {
                    ComponentCheck::NeedName
                }
            }
            Expression::ConditionalExpression(expression) => {
                let consequent = self.is_expression_react_component(&expression.consequent);
                let alternate = self.is_expression_react_component(&expression.alternate);
                if consequent == ComponentCheck::No || alternate == ComponentCheck::No {
                    ComponentCheck::No
                } else if consequent == ComponentCheck::NeedName
                    || alternate == ComponentCheck::NeedName
                {
                    ComponentCheck::NeedName
                } else {
                    ComponentCheck::Yes
                }
            }
            Expression::CallExpression(call) => self.is_call_expression_react_component(call),
            Expression::TaggedTemplateExpression(tagged) => {
                if self.get_tagged_template_hoc_name(tagged).is_some() {
                    ComponentCheck::NeedName
                } else {
                    ComponentCheck::No
                }
            }
            _ => ComponentCheck::No,
        }
    }

    fn is_call_expression_react_component(&self, call: &CallExpression<'_>) -> ComponentCheck {
        let Some(hoc_name) = self.get_call_hoc_name(call) else {
            return ComponentCheck::No;
        };
        if !self.is_valid_hoc(hoc_name) {
            return ComponentCheck::No;
        }

        if hoc_name != "memo" && hoc_name != "forwardRef" {
            return ComponentCheck::Yes;
        }

        let Some(argument) = call.arguments.first() else {
            return ComponentCheck::No;
        };

        self.is_argument_react_component(argument)
    }

    fn is_argument_react_component(&self, argument: &Argument<'_>) -> ComponentCheck {
        match argument {
            Argument::Identifier(identifier) => component_check_for_name(identifier.name.as_str()),
            Argument::FunctionExpression(function) => {
                if let Some(id) = &function.id {
                    component_check_for_name(id.name.as_str())
                } else {
                    ComponentCheck::NeedName
                }
            }
            Argument::ArrowFunctionExpression(function) => {
                if function_param_count(&function.params) > 2 {
                    ComponentCheck::No
                } else {
                    ComponentCheck::NeedName
                }
            }
            Argument::CallExpression(call) => self.is_call_expression_react_component(call),
            Argument::TSAsExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSSatisfiesExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSTypeAssertion(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSNonNullExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSInstantiationExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            _ => ComponentCheck::No,
        }
    }

    fn get_call_hoc_name<'a>(&self, call: &'a CallExpression<'a>) -> Option<&'a str> {
        self.get_hoc_name_from_expression(&call.callee)
    }

    fn get_tagged_template_hoc_name<'a>(
        &self,
        tagged: &'a TaggedTemplateExpression<'a>,
    ) -> Option<&'a str> {
        let name = self.get_hoc_name_from_expression(&tagged.tag)?;
        self.is_valid_hoc(name).then_some(name)
    }

    fn get_hoc_name_from_expression<'a>(&self, expression: &'a Expression<'a>) -> Option<&'a str> {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => self.get_call_hoc_name(call),
            Expression::StaticMemberExpression(member) => {
                let property_name = member.property.name.as_str();
                if self.is_valid_hoc(property_name) {
                    return Some(property_name);
                }
                if let Expression::Identifier(object) = member.object.get_inner_expression() {
                    let object_name = object.name.as_str();
                    if self.is_valid_hoc(object_name) {
                        return Some(object_name);
                    }
                }
                if let Expression::CallExpression(call) = member.object.get_inner_expression() {
                    return self.get_call_hoc_name(call);
                }
                None
            }
            Expression::Identifier(identifier) => Some(identifier.name.as_str()),
            _ => None,
        }
    }

    fn is_valid_hoc(&self, name: &str) -> bool {
        DEFAULT_HOCS.contains(&name) || self.options.extra_hocs.iter().any(|hoc| hoc == name)
    }

    fn report(&mut self, message_id: &'static str, span: Span) {
        self.state.diagnostics.push(Diagnostic {
            message_id,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}
