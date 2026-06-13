//! Statement-level AST traversal for the security scanner.

use oxc_ast::ast::*;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{is_interesting_package, module_export_name, small_path, INTERESTING_PACKAGES};
use crate::scanner::Scanner;
use crate::{AccessPath, Binding, ParentKind};

impl<'a> Scanner<'a> {
    pub(crate) fn scan_statement(&mut self, statement: &'a Statement<'a>) {
        match statement {
            Statement::BlockStatement(block) => {
                for statement in &block.body {
                    self.scan_statement(statement);
                }
            }
            Statement::ExpressionStatement(statement) => {
                self.scan_expression(&statement.expression, ParentKind::None);
            }
            Statement::IfStatement(statement) => {
                self.check_possible_timing_attack(statement.span, &statement.test);
                self.scan_expression(&statement.test, ParentKind::Other);
                self.scan_statement(&statement.consequent);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate);
                }
            }
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument, ParentKind::Other);
                }
            }
            Statement::ThrowStatement(statement) => {
                self.scan_expression(&statement.argument, ParentKind::Other);
            }
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test, ParentKind::Other);
                self.scan_statement(&statement.body);
            }
            Statement::DoWhileStatement(statement) => {
                self.scan_statement(&statement.body);
                self.scan_expression(&statement.test, ParentKind::Other);
            }
            Statement::ForStatement(statement) => {
                if let Some(init) = &statement.init {
                    self.scan_for_init(init);
                }
                if let Some(test) = &statement.test {
                    self.scan_expression(test, ParentKind::Other);
                }
                if let Some(update) = &statement.update {
                    self.scan_expression(update, ParentKind::Other);
                }
                self.scan_statement(&statement.body);
            }
            Statement::ForInStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right, ParentKind::Other);
                self.scan_statement(&statement.body);
            }
            Statement::ForOfStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right, ParentKind::Other);
                self.scan_statement(&statement.body);
            }
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant, ParentKind::Other);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test, ParentKind::Other);
                    }
                    for statement in &case.consequent {
                        self.scan_statement(statement);
                    }
                }
            }
            Statement::TryStatement(statement) => {
                for statement in &statement.block.body {
                    self.scan_statement(statement);
                }
                if let Some(handler) = &statement.handler {
                    self.push_scope();
                    if let Some(param) = &handler.param {
                        self.bind_pattern_unknown(&param.pattern);
                    }
                    for statement in &handler.body.body {
                        self.scan_statement(statement);
                    }
                    self.pop_scope();
                }
                if let Some(finalizer) = &statement.finalizer {
                    for statement in &finalizer.body {
                        self.scan_statement(statement);
                    }
                }
            }
            Statement::LabeledStatement(statement) => {
                self.scan_statement(&statement.body);
            }
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Statement::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_function(function);
            }
            Statement::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_class(class);
            }
            Statement::ImportDeclaration(declaration) => {
                self.scan_import_declaration(declaration);
            }
            Statement::ExportNamedDeclaration(declaration) => {
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration);
                }
            }
            Statement::ExportDefaultDeclaration(declaration) => match &declaration.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                    self.scan_function(function);
                }
                ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                    self.scan_class(class);
                }
                _ => {
                    if let Some(expression) = declaration.declaration.as_expression() {
                        self.scan_expression(expression, ParentKind::Other);
                    }
                }
            },
            _ => {}
        }
    }

    fn scan_declaration(&mut self, declaration: &'a Declaration<'a>) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_function(function);
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_class(class);
            }
            _ => {}
        }
    }

    fn scan_for_init(&mut self, init: &'a ForStatementInit<'a>) {
        match init {
            ForStatementInit::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            _ => {
                if let Some(expression) = init.as_expression() {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
        }
    }

    fn scan_for_left(&mut self, left: &'a ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration);
        }
    }

    fn scan_import_declaration(&mut self, declaration: &'a ImportDeclaration<'a>) {
        let package_name = declaration.source.value.as_str();
        let interesting = is_interesting_package(package_name);

        if let Some(specifiers) = &declaration.specifiers {
            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        let imported = module_export_name(&specifier.imported);
                        let binding = if interesting {
                            imported.map(|name| {
                                Binding::Import(AccessPath {
                                    package_name: CompactString::from(package_name),
                                    path: small_path([name]),
                                })
                            })
                        } else {
                            None
                        }
                        .unwrap_or(Binding::Unknown);
                        self.bind(specifier.local.name.as_str(), binding);
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                        let binding = if interesting {
                            Binding::Import(AccessPath {
                                package_name: CompactString::from(package_name),
                                path: SmallVec::new(),
                            })
                        } else {
                            Binding::Unknown
                        };
                        self.bind(specifier.local.name.as_str(), binding);
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        let binding = if interesting {
                            Binding::Import(AccessPath {
                                package_name: CompactString::from(package_name),
                                path: SmallVec::new(),
                            })
                        } else {
                            Binding::Unknown
                        };
                        self.bind(specifier.local.name.as_str(), binding);
                    }
                }
            }
        }
    }

    fn scan_variable_declaration(&mut self, declaration: &'a VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            if let Some(init) = &declarator.init {
                if let Some(path) = self.import_access_path(init, &INTERESTING_PACKAGES) {
                    self.bind_pattern_from_import(&declarator.id, &path);
                } else if self.is_static_expression(init, 0) {
                    self.bind_pattern_static_or_unknown(&declarator.id, true);
                } else {
                    self.bind_pattern_unknown(&declarator.id);
                }
                self.scan_expression(init, ParentKind::VariableInit);
            } else {
                self.bind_pattern_unknown(&declarator.id);
            }
        }
    }

    pub(crate) fn scan_function(&mut self, function: &'a Function<'a>) {
        self.push_scope();
        for param in &function.params.items {
            self.bind_pattern_unknown(&param.pattern);
            if let Some(initializer) = &param.initializer {
                self.scan_expression(initializer, ParentKind::Other);
            }
        }
        if let Some(rest) = &function.params.rest {
            self.bind_pattern_unknown(&rest.rest.argument);
        }
        if let Some(body) = &function.body {
            self.scan_function_body(body);
        }
        self.pop_scope();
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>) {
        for statement in &body.statements {
            self.scan_statement(statement);
        }
    }

    pub(crate) fn scan_class(&mut self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class, ParentKind::Other);
        }
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    self.push_scope();
                    for statement in &block.body {
                        self.scan_statement(statement);
                    }
                    self.pop_scope();
                }
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value);
                }
                ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ParentKind::Other);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ParentKind::Other);
                    }
                }
                ClassElement::TSIndexSignature(_) => {}
            }
        }
    }
}
