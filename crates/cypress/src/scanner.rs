//! AST scanner for the cypress port. Contains the `Scanner` struct and
//! every traversal / rule check method as an `impl Scanner` block.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrayPattern, ArrowFunctionExpression, AwaitExpression,
    BindingPattern, BindingRestElement, CallExpression, ChainElement, Class, ClassElement,
    ConditionalExpression, Declaration, ExportDefaultDeclarationKind, Expression, ForStatementInit,
    ForStatementLeft, Function, FunctionBody, ImportDeclaration, ImportDeclarationSpecifier,
    ObjectPropertyKind, PropertyKey, Statement, StaticMemberExpression, VariableDeclaration,
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};

use crate::helpers::*;
use crate::{
    ALLOW_AND_AFTER, ASSERTION_COMMANDS, ASSIGNMENT_ALLOWED_COMMANDS, Diagnostic, DiagnosticFix,
    FORCE_ACTION_COMMANDS, LineIndex, ParentKind, Scope, UNSAFE_CHAIN_ACTIONS, ValueKind,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    pub(crate) scopes: SmallVec<[Scope; 8]>,
    pub(crate) data_selector_variables: FastHashMap<CompactString, bool>,
    pub(crate) unsafe_to_chain_methods: SmallVec<[CompactString; 8]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop_scope(&mut self) {
        let _ = self.scopes.pop();
    }

    fn bind_value(&mut self, name: &str, value: ValueKind) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.values.insert(CompactString::from(name), value);
        }
    }

    fn lookup_value(&self, name: &str) -> Option<ValueKind> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.values.get(name).copied())
    }

    fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_fix(rule_name, message_id, span, None);
    }

    fn report_with_fix(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        fix: Option<DiagnosticFix>,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix,
        });
    }

    pub(crate) fn scan_statement_list(
        &mut self,
        statements: &'a [Statement<'a>],
        inherited_previous_command: Option<&str>,
        function_body: bool,
    ) -> Option<CompactString> {
        let mut previous_command = if function_body {
            None
        } else {
            inherited_previous_command.map(CompactString::from)
        };

        for statement in statements {
            previous_command = self.scan_statement(statement, previous_command.as_deref());
        }

        previous_command
    }

    fn scan_statement(
        &mut self,
        statement: &'a Statement<'a>,
        previous_command: Option<&str>,
    ) -> Option<CompactString> {
        match statement {
            Statement::ExpressionStatement(statement) => {
                self.scan_expression(&statement.expression, ParentKind::None, previous_command);
                self.expression_cypress_command(&statement.expression)
                    .map(CompactString::from)
            }
            Statement::BlockStatement(block) => {
                self.push_scope();
                self.scan_statement_list(&block.body, previous_command, false);
                self.pop_scope();
                None
            }
            Statement::IfStatement(statement) => {
                self.scan_expression(&statement.test, ParentKind::Other, previous_command);
                self.scan_statement(&statement.consequent, previous_command);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate, previous_command);
                }
                None
            }
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration, previous_command);
                None
            }
            Statement::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.bind_value(id.name.as_str(), ValueKind::Other);
                }
                self.scan_function(function);
                None
            }
            Statement::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.bind_value(id.name.as_str(), ValueKind::Other);
                }
                self.scan_class(class, previous_command);
                None
            }
            Statement::ImportDeclaration(import) => {
                self.scan_import_declaration(import);
                None
            }
            Statement::ExportNamedDeclaration(declaration) => {
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration, previous_command);
                }
                None
            }
            Statement::ExportDefaultDeclaration(declaration) => {
                match &declaration.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        self.scan_function(function);
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        self.scan_class(class, previous_command);
                    }
                    declaration => {
                        if let Some(expression) = declaration.as_expression() {
                            self.scan_expression(expression, ParentKind::None, previous_command);
                        }
                    }
                }
                None
            }
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument, ParentKind::Other, previous_command);
                }
                None
            }
            Statement::ThrowStatement(statement) => {
                self.scan_expression(&statement.argument, ParentKind::Other, previous_command);
                None
            }
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test, ParentKind::Other, previous_command);
                self.scan_statement(&statement.body, previous_command);
                None
            }
            Statement::DoWhileStatement(statement) => {
                self.scan_statement(&statement.body, previous_command);
                self.scan_expression(&statement.test, ParentKind::Other, previous_command);
                None
            }
            Statement::ForStatement(statement) => {
                self.push_scope();
                if let Some(init) = &statement.init {
                    self.scan_for_statement_init(init, previous_command);
                }
                if let Some(test) = &statement.test {
                    self.scan_expression(test, ParentKind::Other, previous_command);
                }
                if let Some(update) = &statement.update {
                    self.scan_expression(update, ParentKind::Other, previous_command);
                }
                self.scan_statement(&statement.body, previous_command);
                self.pop_scope();
                None
            }
            Statement::ForInStatement(statement) => {
                self.push_scope();
                self.scan_for_statement_left(&statement.left);
                self.scan_expression(&statement.right, ParentKind::Other, previous_command);
                self.scan_statement(&statement.body, previous_command);
                self.pop_scope();
                None
            }
            Statement::ForOfStatement(statement) => {
                self.push_scope();
                self.scan_for_statement_left(&statement.left);
                self.scan_expression(&statement.right, ParentKind::Other, previous_command);
                self.scan_statement(&statement.body, previous_command);
                self.pop_scope();
                None
            }
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant, ParentKind::Other, previous_command);
                self.push_scope();
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test, ParentKind::Other, previous_command);
                    }
                    self.scan_statement_list(&case.consequent, None, false);
                }
                self.pop_scope();
                None
            }
            Statement::TryStatement(statement) => {
                self.scan_statement_list(&statement.block.body, previous_command, false);
                if let Some(handler) = &statement.handler {
                    self.push_scope();
                    if let Some(param) = &handler.param {
                        self.bind_pattern(&param.pattern, ValueKind::Other);
                    }
                    self.scan_statement_list(&handler.body.body, None, false);
                    self.pop_scope();
                }
                if let Some(finalizer) = &statement.finalizer {
                    self.scan_statement_list(&finalizer.body, None, false);
                }
                None
            }
            Statement::LabeledStatement(statement) => {
                self.scan_statement(&statement.body, previous_command);
                None
            }
            Statement::WithStatement(statement) => {
                self.scan_expression(&statement.object, ParentKind::Other, previous_command);
                self.scan_statement(&statement.body, previous_command);
                None
            }
            Statement::TSExportAssignment(statement) => {
                self.scan_expression(&statement.expression, ParentKind::Other, previous_command);
                None
            }
            _ => None,
        }
    }

    fn scan_declaration(
        &mut self,
        declaration: &'a Declaration<'a>,
        previous_command: Option<&str>,
    ) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration, previous_command);
            }
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.bind_value(id.name.as_str(), ValueKind::Other);
                }
                self.scan_function(function);
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.bind_value(id.name.as_str(), ValueKind::Other);
                }
                self.scan_class(class, previous_command);
            }
            _ => {}
        }
    }

    fn scan_import_declaration(&mut self, declaration: &'a ImportDeclaration<'a>) {
        if let Some(specifiers) = &declaration.specifiers {
            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        self.bind_value(specifier.local.name.as_str(), ValueKind::Other);
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                        self.bind_value(specifier.local.name.as_str(), ValueKind::Other);
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        self.bind_value(specifier.local.name.as_str(), ValueKind::Other);
                    }
                }
            }
        }
    }

    fn scan_for_statement_init(
        &mut self,
        init: &'a ForStatementInit<'a>,
        previous_command: Option<&str>,
    ) {
        match init {
            ForStatementInit::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration, previous_command);
            }
            ForStatementInit::CallExpression(expression) => {
                self.scan_call_expression(expression, ParentKind::Other, previous_command);
            }
            ForStatementInit::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, previous_command);
            }
            ForStatementInit::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
                self.scan_expression(&member.expression, ParentKind::Other, previous_command);
            }
            ForStatementInit::AssignmentExpression(expression) => {
                self.scan_expression(&expression.right, ParentKind::Other, previous_command);
            }
            ForStatementInit::SequenceExpression(expression) => {
                for expression in &expression.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            _ => {}
        }
    }

    fn scan_for_statement_left(&mut self, left: &'a ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            for declarator in &declaration.declarations {
                self.bind_pattern(&declarator.id, ValueKind::Other);
            }
        }
    }

    fn scan_variable_declaration(
        &mut self,
        declaration: &'a VariableDeclaration<'a>,
        previous_command: Option<&str>,
    ) {
        if declaration
            .declarations
            .iter()
            .any(|declarator| self.is_cypress_command_declaration(declarator.init.as_ref()))
        {
            self.report("no-assigning-return-values", "unexpected", declaration.span);
        }

        for declarator in &declaration.declarations {
            if let Some(init) = &declarator.init {
                if let BindingPattern::BindingIdentifier(id) = &declarator.id
                    && self.is_data_node_expression(init)
                {
                    self.data_selector_variables
                        .insert(CompactString::from(id.name.as_str()), true);
                }

                let value = if self.is_numeric_expression(init) {
                    ValueKind::Number
                } else {
                    ValueKind::Other
                };
                self.bind_pattern(&declarator.id, value);
                self.scan_expression(init, ParentKind::Other, previous_command);
            } else {
                self.bind_pattern(&declarator.id, ValueKind::Other);
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &'a BindingPattern<'a>, value: ValueKind) {
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                self.bind_value(identifier.name.as_str(), value);
            }
            BindingPattern::ObjectPattern(pattern) => {
                for property in &pattern.properties {
                    self.bind_pattern(&property.value, value);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_rest(rest, value);
                }
            }
            BindingPattern::ArrayPattern(pattern) => {
                self.bind_array_pattern(pattern, value);
            }
            BindingPattern::AssignmentPattern(pattern) => {
                let value = if self.is_numeric_expression(&pattern.right) {
                    ValueKind::Number
                } else {
                    value
                };
                self.bind_pattern(&pattern.left, value);
            }
        }
    }

    fn bind_array_pattern(&mut self, pattern: &'a ArrayPattern<'a>, value: ValueKind) {
        for element in pattern.elements.iter().flatten() {
            self.bind_pattern(element, value);
        }
        if let Some(rest) = &pattern.rest {
            self.bind_rest(rest, value);
        }
    }

    fn bind_rest(&mut self, rest: &'a BindingRestElement<'a>, value: ValueKind) {
        self.bind_pattern(&rest.argument, value);
    }

    fn scan_function(&mut self, function: &'a Function<'a>) {
        self.push_scope();
        self.bind_function_params(&function.params);
        if let Some(body) = &function.body {
            self.scan_function_body(body);
        }
        self.pop_scope();
    }

    fn scan_arrow_function(&mut self, function: &'a ArrowFunctionExpression<'a>) {
        self.push_scope();
        self.bind_function_params(&function.params);
        self.scan_function_body(&function.body);
        self.pop_scope();
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>) {
        self.scan_statement_list(&body.statements, None, true);
    }

    fn bind_function_params(&mut self, params: &'a oxc_ast::ast::FormalParameters<'a>) {
        for param in &params.items {
            let value = param
                .initializer
                .as_deref()
                .map(|initializer| {
                    if self.is_numeric_expression(initializer) {
                        ValueKind::Number
                    } else {
                        ValueKind::Other
                    }
                })
                .unwrap_or(ValueKind::Other);
            self.bind_pattern(&param.pattern, value);
        }
        if let Some(rest) = &params.rest {
            self.bind_pattern(&rest.rest.argument, ValueKind::Other);
        }
    }

    fn scan_class(&mut self, class: &'a Class<'a>, previous_command: Option<&str>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class, ParentKind::Other, previous_command);
        }
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    self.scan_statement_list(&block.body, None, false);
                }
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value);
                }
                ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ParentKind::Other, previous_command);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ParentKind::Other, previous_command);
                    }
                }
                ClassElement::TSIndexSignature(_) => {}
            }
        }
    }

    fn scan_expression(
        &mut self,
        expression: &'a Expression<'a>,
        parent_kind: ParentKind,
        previous_command: Option<&str>,
    ) {
        match expression {
            Expression::CallExpression(call) => {
                self.scan_call_expression(call, parent_kind, previous_command);
            }
            Expression::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, previous_command);
            }
            Expression::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
                self.scan_expression(&member.expression, ParentKind::Other, previous_command);
            }
            Expression::ChainExpression(chain) => {
                self.scan_chain_element(&chain.expression, parent_kind, previous_command);
            }
            Expression::ParenthesizedExpression(expression) => {
                self.scan_expression(&expression.expression, parent_kind, previous_command);
            }
            Expression::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, parent_kind, previous_command);
            }
            Expression::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, parent_kind, previous_command);
            }
            Expression::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, parent_kind, previous_command);
            }
            Expression::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, parent_kind, previous_command);
            }
            Expression::TSInstantiationExpression(expression) => {
                self.scan_expression(&expression.expression, parent_kind, previous_command);
            }
            Expression::AwaitExpression(expression) => {
                self.scan_await_expression(expression, previous_command);
            }
            Expression::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, previous_command);
                }
            }
            Expression::ObjectExpression(expression) => {
                for property in &expression.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed {
                                self.scan_property_key(&property.key, previous_command);
                            }
                            self.scan_expression(
                                &property.value,
                                ParentKind::Other,
                                previous_command,
                            );
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(
                                &spread.argument,
                                ParentKind::Other,
                                previous_command,
                            );
                        }
                    }
                }
            }
            Expression::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function);
            }
            Expression::FunctionExpression(function) => {
                self.scan_function(function);
            }
            Expression::ClassExpression(class) => {
                self.scan_class(class, previous_command);
            }
            Expression::AssignmentExpression(expression) => {
                self.scan_expression(&expression.right, ParentKind::Other, previous_command);
            }
            Expression::ConditionalExpression(expression) => {
                self.scan_conditional_expression(expression, previous_command);
            }
            Expression::BinaryExpression(expression) => {
                self.scan_expression(&expression.left, ParentKind::Other, previous_command);
                self.scan_expression(&expression.right, ParentKind::Other, previous_command);
            }
            Expression::LogicalExpression(expression) => {
                self.scan_expression(&expression.left, ParentKind::Other, previous_command);
                self.scan_expression(&expression.right, ParentKind::Other, previous_command);
            }
            Expression::SequenceExpression(expression) => {
                for expression in &expression.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            Expression::UnaryExpression(expression) => {
                self.scan_expression(&expression.argument, ParentKind::Other, previous_command);
            }
            Expression::UpdateExpression(_) => {}
            Expression::YieldExpression(expression) => {
                if let Some(argument) = &expression.argument {
                    self.scan_expression(argument, ParentKind::Other, previous_command);
                }
            }
            Expression::TaggedTemplateExpression(expression) => {
                self.scan_expression(&expression.tag, ParentKind::Other, previous_command);
                for expression in &expression.quasi.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            Expression::ImportExpression(expression) => {
                self.scan_expression(&expression.source, ParentKind::Other, previous_command);
                if let Some(options) = &expression.options {
                    self.scan_expression(options, ParentKind::Other, previous_command);
                }
            }
            _ => {}
        }
    }

    fn scan_chain_element(
        &mut self,
        element: &'a ChainElement<'a>,
        parent_kind: ParentKind,
        previous_command: Option<&str>,
    ) {
        match element {
            ChainElement::CallExpression(call) => {
                self.scan_call_expression(call, parent_kind, previous_command);
            }
            ChainElement::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, previous_command);
            }
            ChainElement::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
                self.scan_expression(&member.expression, ParentKind::Other, previous_command);
            }
            ChainElement::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, parent_kind, previous_command);
            }
            ChainElement::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
            }
        }
    }

    fn scan_static_member_expression(
        &mut self,
        member: &'a StaticMemberExpression<'a>,
        previous_command: Option<&str>,
    ) {
        self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
    }

    fn scan_await_expression(
        &mut self,
        expression: &'a AwaitExpression<'a>,
        previous_command: Option<&str>,
    ) {
        self.scan_expression(&expression.argument, ParentKind::Other, previous_command);
    }

    fn scan_conditional_expression(
        &mut self,
        expression: &'a ConditionalExpression<'a>,
        previous_command: Option<&str>,
    ) {
        self.scan_expression(&expression.test, ParentKind::Other, previous_command);
        self.scan_expression(&expression.consequent, ParentKind::Other, previous_command);
        self.scan_expression(&expression.alternate, ParentKind::Other, previous_command);
    }

    fn scan_array_element(
        &mut self,
        element: &'a ArrayExpressionElement<'a>,
        previous_command: Option<&str>,
    ) {
        match element {
            ArrayExpressionElement::SpreadElement(spread) => {
                self.scan_expression(&spread.argument, ParentKind::Other, previous_command);
            }
            ArrayExpressionElement::CallExpression(call) => {
                self.scan_call_expression(call, ParentKind::Other, previous_command);
            }
            ArrayExpressionElement::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, previous_command);
            }
            ArrayExpressionElement::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
                self.scan_expression(&member.expression, ParentKind::Other, previous_command);
            }
            ArrayExpressionElement::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function);
            }
            ArrayExpressionElement::FunctionExpression(function) => {
                self.scan_function(function);
            }
            ArrayExpressionElement::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property {
                        self.scan_expression(&property.value, ParentKind::Other, previous_command);
                    }
                }
            }
            ArrayExpressionElement::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, previous_command);
                }
            }
            ArrayExpressionElement::ConditionalExpression(expression) => {
                self.scan_conditional_expression(expression, previous_command);
            }
            ArrayExpressionElement::Elision(_) => {}
            _ => {}
        }
    }

    fn scan_argument(&mut self, argument: &'a Argument<'a>, previous_command: Option<&str>) {
        match argument {
            Argument::SpreadElement(spread) => {
                self.scan_expression(&spread.argument, ParentKind::Other, previous_command);
            }
            Argument::CallExpression(call) => {
                self.scan_call_expression(call, ParentKind::Other, previous_command);
            }
            Argument::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, previous_command);
            }
            Argument::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
                self.scan_expression(&member.expression, ParentKind::Other, previous_command);
            }
            Argument::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function);
            }
            Argument::FunctionExpression(function) => {
                self.scan_function(function);
            }
            Argument::ObjectExpression(expression) => {
                for property in &expression.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed {
                                self.scan_property_key(&property.key, previous_command);
                            }
                            self.scan_expression(
                                &property.value,
                                ParentKind::Other,
                                previous_command,
                            );
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(
                                &spread.argument,
                                ParentKind::Other,
                                previous_command,
                            );
                        }
                    }
                }
            }
            Argument::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, previous_command);
                }
            }
            Argument::ConditionalExpression(expression) => {
                self.scan_conditional_expression(expression, previous_command);
            }
            Argument::AwaitExpression(expression) => {
                self.scan_await_expression(expression, previous_command);
            }
            Argument::SequenceExpression(expression) => {
                for expression in &expression.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            Argument::TaggedTemplateExpression(expression) => {
                self.scan_expression(&expression.tag, ParentKind::Other, previous_command);
                for expression in &expression.quasi.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            Argument::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            _ => {}
        }
    }

    fn scan_property_key(&mut self, key: &'a PropertyKey<'a>, previous_command: Option<&str>) {
        match key {
            PropertyKey::CallExpression(call) => {
                self.scan_call_expression(call, ParentKind::Other, previous_command);
            }
            PropertyKey::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, previous_command);
            }
            PropertyKey::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject, previous_command);
                self.scan_expression(&member.expression, ParentKind::Other, previous_command);
            }
            PropertyKey::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression, ParentKind::Other, previous_command);
                }
            }
            _ => {}
        }
    }

    fn scan_call_expression(
        &mut self,
        call: &'a CallExpression<'a>,
        parent_kind: ParentKind,
        previous_command: Option<&str>,
    ) {
        self.check_call_rules(call, parent_kind, previous_command);

        self.scan_expression(&call.callee, ParentKind::Other, previous_command);
        for argument in &call.arguments {
            self.scan_argument(argument, previous_command);
        }
    }

    fn check_call_rules(
        &mut self,
        call: &'a CallExpression<'a>,
        parent_kind: ParentKind,
        previous_command: Option<&str>,
    ) {
        let Some(command) = call_static_member_name(call) else {
            self.check_async_block_rules(call);
            return;
        };

        if self.is_root_cypress_call(call) {
            match command {
                "screenshot" => {
                    if !self.previous_is_assertion(call, previous_command) {
                        self.report("assertion-before-screenshot", "unexpected", call.span);
                    }
                }
                "and" => {
                    if !self.is_allowed_and_call(call)
                        && let Expression::StaticMemberExpression(member) = &call.callee
                    {
                        self.report_with_fix(
                            "no-and",
                            "unexpected",
                            call.span,
                            Some(DiagnosticFix {
                                start: member.property.span.start,
                                end: member.property.span.end,
                                replacement: "should",
                            }),
                        );
                    }
                }
                "get" => {
                    if self.has_chained_get(call) {
                        self.report("no-chained-get", "unexpected", call.span);
                    }
                }
                "debug" => {
                    self.report("no-debug", "unexpected", call.span);
                }
                "pause" => {
                    self.report("no-pause", "unexpected", call.span);
                }
                "wait" if self.waits_for_number(call) => {
                    self.report("no-unnecessary-waiting", "unexpected", call.span);
                }
                _ => {}
            }

            if self.is_force_action(command) && call_has_force_option(call) {
                self.report("no-force", "unexpected", call.span);
            }

            if parent_kind == ParentKind::MemberObject && self.is_unsafe_chain_action(command) {
                self.report("unsafe-to-chain-command", "unexpected", call.span);
            }
        }

        if command == "xpath" && self.is_direct_cy_call(call) {
            self.report("no-xpath", "unexpected", call.span);
        }

        if command == "get" && self.is_direct_cy_call(call) && !self.get_uses_data_selector(call) {
            self.report("require-data-selectors", "unexpected", call.span);
        }

        self.check_async_block_rules(call);
    }

    fn check_async_block_rules(&mut self, call: &'a CallExpression<'a>) {
        let Some(callee_name) = callee_identifier_name(call) else {
            return;
        };
        let Some(callback) = call.arguments.get(1) else {
            return;
        };

        let callback_is_async = argument_is_async_function(callback);
        if !callback_is_async || !argument_contains_cypress_identifier(callback) {
            return;
        }

        match callee_name {
            "before" | "beforeEach" => {
                self.report("no-async-before", "unexpected", call.span);
            }
            "it" | "test" => {
                self.report("no-async-tests", "unexpected", call.span);
            }
            _ => {}
        }
    }

    fn previous_is_assertion(
        &self,
        call: &'a CallExpression<'a>,
        previous_command: Option<&str>,
    ) -> bool {
        let previous = previous_command_in_chain(call).or(previous_command);
        previous.is_some_and(|command| ASSERTION_COMMANDS.contains(&command))
    }

    fn is_allowed_and_call(&self, call: &'a CallExpression<'a>) -> bool {
        previous_command_in_chain(call).is_some_and(|command| ALLOW_AND_AFTER.contains(&command))
    }

    fn has_chained_get(&self, call: &'a CallExpression<'a>) -> bool {
        if call_static_member_name(call) != Some("get") {
            return false;
        }

        let mut object = call_static_member_object(call);
        while let Some(Expression::CallExpression(object_call)) = object {
            if call_static_member_name(object_call) == Some("get") {
                return true;
            }
            object = call_static_member_object(object_call);
        }

        false
    }

    fn is_root_cypress_call(&self, call: &'a CallExpression<'a>) -> bool {
        match &call.callee {
            Expression::StaticMemberExpression(member) => {
                if expression_identifier_name(&member.object) == Some("cy") {
                    return true;
                }
                if let Expression::CallExpression(object_call) = &member.object {
                    return self.is_root_cypress_call(object_call);
                }
                false
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::StaticMemberExpression(member) => {
                    if expression_identifier_name(&member.object) == Some("cy") {
                        return true;
                    }
                    if let Expression::CallExpression(object_call) = &member.object {
                        return self.is_root_cypress_call(object_call);
                    }
                    false
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn is_direct_cy_call(&self, call: &'a CallExpression<'a>) -> bool {
        matches!(
            &call.callee,
            Expression::StaticMemberExpression(member)
                if expression_identifier_name(&member.object) == Some("cy")
        )
    }

    fn is_force_action(&self, command: &str) -> bool {
        FORCE_ACTION_COMMANDS.contains(&command)
    }

    fn is_unsafe_chain_action(&self, command: &str) -> bool {
        UNSAFE_CHAIN_ACTIONS.contains(&command)
            || self
                .unsafe_to_chain_methods
                .iter()
                .any(|method| method.as_str() == command)
    }

    fn waits_for_number(&self, call: &'a CallExpression<'a>) -> bool {
        let Some(argument) = call.arguments.first() else {
            return false;
        };

        match argument {
            Argument::NumericLiteral(_) => true,
            Argument::Identifier(identifier) => {
                self.lookup_value(identifier.name.as_str()) == Some(ValueKind::Number)
            }
            _ => false,
        }
    }

    fn get_uses_data_selector(&self, call: &'a CallExpression<'a>) -> bool {
        call.arguments
            .first()
            .is_some_and(|argument| self.is_data_node_argument(argument))
    }

    fn is_data_node_argument(&self, argument: &'a Argument<'a>) -> bool {
        match argument {
            Argument::StringLiteral(literal) => is_alias_or_data_selector(literal.value.as_str()),
            Argument::TemplateLiteral(template) => template
                .quasis
                .first()
                .and_then(|quasi| quasi.value.cooked.as_ref())
                .is_some_and(|value| is_alias_or_data_selector(value.as_str())),
            Argument::Identifier(identifier) => self
                .data_selector_variables
                .contains_key(identifier.name.as_str()),
            Argument::ConditionalExpression(expression) => {
                self.is_data_node_expression(&expression.consequent)
                    && self.is_data_node_expression(&expression.alternate)
            }
            _ => false,
        }
    }

    fn is_data_node_expression(&self, expression: &'a Expression<'a>) -> bool {
        match expression.get_inner_expression() {
            Expression::StringLiteral(literal) => is_alias_or_data_selector(literal.value.as_str()),
            Expression::TemplateLiteral(template) => template
                .quasis
                .first()
                .and_then(|quasi| quasi.value.cooked.as_ref())
                .is_some_and(|value| is_alias_or_data_selector(value.as_str())),
            Expression::Identifier(identifier) => self
                .data_selector_variables
                .contains_key(identifier.name.as_str()),
            Expression::ConditionalExpression(expression) => {
                self.is_data_node_expression(&expression.consequent)
                    && self.is_data_node_expression(&expression.alternate)
            }
            _ => false,
        }
    }

    fn is_numeric_expression(&self, expression: &'a Expression<'a>) -> bool {
        matches!(
            expression.get_inner_expression(),
            Expression::NumericLiteral(_)
        )
    }

    fn is_cypress_command_declaration(&self, init: Option<&'a Expression<'a>>) -> bool {
        let Some(Expression::CallExpression(call)) = init.map(Expression::get_inner_expression)
        else {
            return false;
        };

        let Some((first_command, last_command)) = cypress_command_names(call) else {
            return false;
        };

        if ASSIGNMENT_ALLOWED_COMMANDS.contains(&first_command)
            || ASSIGNMENT_ALLOWED_COMMANDS.contains(&last_command)
        {
            return false;
        }

        true
    }

    fn expression_cypress_command(&self, expression: &'a Expression<'a>) -> Option<&'a str> {
        let Expression::CallExpression(call) = expression.get_inner_expression() else {
            return None;
        };
        if !self.is_root_cypress_call(call) {
            return None;
        }
        call_static_member_name(call)
    }
}
