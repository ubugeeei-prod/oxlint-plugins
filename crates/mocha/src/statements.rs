//! Statement-level AST traversal for the mocha scanner.

use oxc_ast::ast::*;

use crate::ContextKind;
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_statement(&mut self, statement: &'a Statement<'a>, context: ContextKind) {
        match statement {
            Statement::ExpressionStatement(statement) => {
                self.scan_expression(&statement.expression, context);
            }
            Statement::BlockStatement(block) => {
                self.scan_statement_list(&block.body, context, false);
            }
            Statement::IfStatement(statement) => {
                self.scan_expression(&statement.test, context);
                self.scan_statement(&statement.consequent, context);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate, context);
                }
            }
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument, context);
                }
            }
            Statement::ThrowStatement(statement) => {
                self.scan_expression(&statement.argument, context);
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if let Some(init) = &declarator.init {
                        self.scan_expression(init, context);
                    }
                }
            }
            Statement::FunctionDeclaration(function) => {
                self.scan_function(function);
            }
            Statement::ClassDeclaration(class) => {
                self.scan_class(class);
            }
            Statement::ExportNamedDeclaration(declaration) => {
                self.export_spans.push(declaration.span);
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration, context);
                }
            }
            Statement::ExportDefaultDeclaration(declaration) => {
                self.export_spans.push(declaration.span);
                match &declaration.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        self.scan_function(function);
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        self.scan_class(class);
                    }
                    declaration => {
                        if let Some(expression) = declaration.as_expression() {
                            self.scan_expression(expression, context);
                        }
                    }
                }
            }
            Statement::ExportAllDeclaration(declaration) => {
                self.export_spans.push(declaration.span);
            }
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test, context);
                self.scan_statement(&statement.body, context);
            }
            Statement::DoWhileStatement(statement) => {
                self.scan_statement(&statement.body, context);
                self.scan_expression(&statement.test, context);
            }
            Statement::ForStatement(statement) => {
                if let Some(test) = &statement.test {
                    self.scan_expression(test, context);
                }
                if let Some(update) = &statement.update {
                    self.scan_expression(update, context);
                }
                self.scan_statement(&statement.body, context);
            }
            Statement::ForInStatement(statement) => {
                self.scan_expression(&statement.right, context);
                self.scan_statement(&statement.body, context);
            }
            Statement::ForOfStatement(statement) => {
                self.scan_expression(&statement.right, context);
                self.scan_statement(&statement.body, context);
            }
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant, context);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test, context);
                    }
                    self.scan_statement_list(&case.consequent, context, false);
                }
            }
            Statement::TryStatement(statement) => {
                self.scan_statement_list(&statement.block.body, context, false);
                if let Some(handler) = &statement.handler {
                    self.scan_statement_list(&handler.body.body, context, false);
                }
                if let Some(finalizer) = &statement.finalizer {
                    self.scan_statement_list(&finalizer.body, context, false);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn scan_declaration(
        &mut self,
        declaration: &'a Declaration<'a>,
        context: ContextKind,
    ) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if let Some(init) = &declarator.init {
                        self.scan_expression(init, context);
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => self.scan_function(function),
            Declaration::ClassDeclaration(class) => self.scan_class(class),
            _ => {}
        }
    }
}
