//! Statement-level AST traversal for the e18e scanner.

use oxc_ast::ast::{
    ClassElement, Declaration, ForStatementInit, ForStatementLeft, Function, FunctionBody,
    Statement, Class,
};

use crate::helpers::ExprContext;
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_statement(&mut self, statement: &'a Statement<'a>) {
        match statement {
            Statement::BlockStatement(block) => {
                for statement in &block.body {
                    self.scan_statement(statement);
                }
            }
            Statement::ExpressionStatement(statement) => {
                self.scan_expression(&statement.expression, ExprContext::Statement);
            }
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration)
            }
            Statement::FunctionDeclaration(function) => self.scan_function(function),
            Statement::ClassDeclaration(class) => self.scan_class(class),
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument, ExprContext::Return);
                }
            }
            Statement::IfStatement(statement) => {
                self.check_prefer_nullish_assignment(statement);
                self.scan_expression(&statement.test, ExprContext::Boolean);
                self.scan_statement(&statement.consequent);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate);
                }
            }
            Statement::ForStatement(statement) => {
                if let Some(init) = &statement.init {
                    match init {
                        ForStatementInit::VariableDeclaration(declaration) => {
                            self.scan_variable_declaration(declaration);
                        }
                        _ => {
                            if let Some(expression) = init.as_expression() {
                                self.scan_expression(expression, ExprContext::Other);
                            }
                        }
                    }
                }
                if let Some(test) = &statement.test {
                    self.scan_expression(test, ExprContext::Boolean);
                }
                if let Some(update) = &statement.update {
                    self.scan_expression(update, ExprContext::Other);
                }
                self.scan_statement(&statement.body);
            }
            Statement::ForInStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right, ExprContext::Other);
                self.scan_statement(&statement.body);
            }
            Statement::ForOfStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right, ExprContext::Other);
                self.scan_statement(&statement.body);
            }
            Statement::DoWhileStatement(statement) => {
                self.scan_statement(&statement.body);
                self.scan_expression(&statement.test, ExprContext::Boolean);
            }
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test, ExprContext::Boolean);
                self.scan_statement(&statement.body);
            }
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant, ExprContext::Other);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test, ExprContext::Other);
                    }
                    for consequent in &case.consequent {
                        self.scan_statement(consequent);
                    }
                }
            }
            Statement::ThrowStatement(statement) => {
                self.scan_expression(&statement.argument, ExprContext::Other);
            }
            Statement::TryStatement(statement) => {
                self.check_prefer_url_canparse(statement);
                for statement in &statement.block.body {
                    self.scan_statement(statement);
                }
                if let Some(handler) = &statement.handler {
                    for statement in &handler.body.body {
                        self.scan_statement(statement);
                    }
                }
                if let Some(finalizer) = &statement.finalizer {
                    for statement in &finalizer.body {
                        self.scan_statement(statement);
                    }
                }
            }
            Statement::WithStatement(statement) => {
                self.scan_expression(&statement.object, ExprContext::Other);
                self.scan_statement(&statement.body);
            }
            Statement::ImportDeclaration(import) => self.check_ban_dependency_import(import),
            Statement::ExportNamedDeclaration(export) => {
                if let Some(source) = &export.source {
                    self.check_ban_dependency_source(source.value.as_str(), source.span);
                }
                if let Some(declaration) = &export.declaration {
                    self.scan_declaration(declaration);
                }
            }
            Statement::ExportAllDeclaration(export) => {
                self.check_ban_dependency_source(export.source.value.as_str(), export.source.span);
            }
            Statement::ExportDefaultDeclaration(export) => match &export.declaration {
                oxc_ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                    self.scan_function(function);
                }
                oxc_ast::ast::ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                    self.scan_class(class);
                }
                _ if export.declaration.as_expression().is_some() => {
                    let expression = export.declaration.as_expression().expect("checked above");
                    self.scan_expression(expression, ExprContext::Other);
                }
                _ => {}
            },
            Statement::LabeledStatement(statement) => self.scan_statement(&statement.body),
            _ => {}
        }
    }

    pub(crate) fn scan_declaration(&mut self, declaration: &'a Declaration<'a>) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Declaration::FunctionDeclaration(function) => self.scan_function(function),
            Declaration::ClassDeclaration(class) => self.scan_class(class),
            _ => {}
        }
    }

    pub(crate) fn scan_variable_declaration(
        &mut self,
        declaration: &'a oxc_ast::ast::VariableDeclaration<'a>,
    ) {
        for declarator in &declaration.declarations {
            if let Some(init) = &declarator.init {
                self.scan_expression(init, ExprContext::Other);
            }
        }
    }

    pub(crate) fn scan_for_left(&mut self, left: &'a ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration);
        }
    }

    pub(crate) fn scan_function(&mut self, function: &'a Function<'a>) {
        self.function_depth += 1;
        if let Some(body) = &function.body {
            self.scan_function_body(body);
        }
        self.function_depth -= 1;
    }

    pub(crate) fn scan_function_body(&mut self, body: &'a FunctionBody<'a>) {
        for statement in &body.statements {
            self.scan_statement(statement);
        }
    }

    pub(crate) fn scan_class(&mut self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class, ExprContext::Other);
        }
        for element in &class.body.body {
            match element {
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value);
                }
                ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ExprContext::Other);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ExprContext::Other);
                    }
                }
                ClassElement::StaticBlock(block) => {
                    for statement in &block.body {
                        self.scan_statement(statement);
                    }
                }
                _ => {}
            }
        }
    }
}
