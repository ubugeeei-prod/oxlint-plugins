//! Statement-level AST traversal for the react-hooks scanner.

use oxc_ast::ast::*;

use crate::helpers::{binding_pattern_name, function_name};
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_statement(&mut self, statement: &'a Statement<'a>) {
        match statement {
            Statement::BlockStatement(block) => {
                self.scan_statement_list(&block.body);
            }
            Statement::ExpressionStatement(statement) => {
                self.scan_expression(&statement.expression);
            }
            Statement::IfStatement(statement) => {
                self.scan_expression(&statement.test);
                self.with_conditional(|scanner| scanner.scan_statement(&statement.consequent));
                if let Some(alternate) = &statement.alternate {
                    self.with_conditional(|scanner| scanner.scan_statement(alternate));
                }
            }
            Statement::ReturnStatement(statement) => {
                if self
                    .current_frame()
                    .is_some_and(|frame| frame.conditional_depth > 0)
                    && let Some(frame) = self.current_frame_mut()
                {
                    frame.possible_early_return = true;
                }
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument);
                }
            }
            Statement::ThrowStatement(statement) => {
                self.scan_expression(&statement.argument);
            }
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test);
                self.with_loop(|scanner| scanner.scan_statement(&statement.body));
            }
            Statement::DoWhileStatement(statement) => {
                self.with_loop(|scanner| scanner.scan_statement(&statement.body));
                self.scan_expression(&statement.test);
            }
            Statement::ForStatement(statement) => {
                if let Some(init) = &statement.init {
                    self.scan_for_init(init);
                }
                self.with_loop(|scanner| {
                    if let Some(test) = &statement.test {
                        scanner.scan_expression(test);
                    }
                    if let Some(update) = &statement.update {
                        scanner.scan_expression(update);
                    }
                    scanner.scan_statement(&statement.body);
                });
            }
            Statement::ForInStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right);
                self.with_loop(|scanner| scanner.scan_statement(&statement.body));
            }
            Statement::ForOfStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right);
                self.with_loop(|scanner| scanner.scan_statement(&statement.body));
            }
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test);
                    }
                    self.with_conditional(|scanner| scanner.scan_statement_list(&case.consequent));
                }
            }
            Statement::TryStatement(statement) => {
                self.with_try(|scanner| scanner.scan_statement_list(&statement.block.body));
                if let Some(handler) = &statement.handler {
                    self.with_try(|scanner| scanner.scan_statement_list(&handler.body.body));
                }
                if let Some(finalizer) = &statement.finalizer {
                    self.with_try(|scanner| scanner.scan_statement_list(&finalizer.body));
                }
            }
            Statement::LabeledStatement(statement) => {
                self.scan_statement(&statement.body);
            }
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Statement::FunctionDeclaration(function) => {
                self.scan_function(function, function_name(function), false);
            }
            Statement::ClassDeclaration(class) => {
                self.scan_class(class);
            }
            Statement::ExportNamedDeclaration(declaration) => {
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration);
                }
            }
            Statement::ExportDefaultDeclaration(declaration) => match &declaration.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                    self.scan_function(function, function_name(function), false);
                }
                ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                    self.scan_class(class);
                }
                _ => {
                    if let Some(expression) = declaration.declaration.as_expression() {
                        self.scan_expression(expression);
                    }
                }
            },
            _ => {}
        }
    }

    pub(crate) fn scan_declaration(&mut self, declaration: &'a Declaration<'a>) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Declaration::FunctionDeclaration(function) => {
                self.scan_function(function, function_name(function), false);
            }
            Declaration::ClassDeclaration(class) => {
                self.scan_class(class);
            }
            _ => {}
        }
    }

    pub(crate) fn scan_for_init(&mut self, init: &'a ForStatementInit<'a>) {
        match init {
            ForStatementInit::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            _ => {
                if let Some(expression) = init.as_expression() {
                    self.scan_expression(expression);
                }
            }
        }
    }

    pub(crate) fn scan_for_left(&mut self, left: &'a ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration);
        }
    }

    pub(crate) fn scan_variable_declaration(
        &mut self,
        declaration: &'a VariableDeclaration<'a>,
    ) {
        for declarator in &declaration.declarations {
            let name = binding_pattern_name(&declarator.id);
            if let Some(init) = &declarator.init {
                if let Expression::ArrowFunctionExpression(function) = init.get_inner_expression() {
                    self.scan_arrow_function(function, name, false);
                } else if let Expression::FunctionExpression(function) = init.get_inner_expression()
                {
                    self.scan_function(function, name.or_else(|| function_name(function)), false);
                } else {
                    self.scan_expression(init);
                }
            }
        }
    }
}
