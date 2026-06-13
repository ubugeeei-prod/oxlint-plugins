//! Statement-level AST traversal for the regexp scanner. Expression-level
//! traversal lives in `expressions.rs`; regexp-specific checks in `checks.rs`.

use oxc_ast::ast::{
    Class, ClassElement, Declaration, ExportDefaultDeclarationKind, ForStatementInit,
    ForStatementLeft, Function, FunctionBody, Statement, VariableDeclaration,
};

use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_program(&mut self, body: &'a [Statement<'a>]) {
        for statement in body {
            self.scan_statement(statement);
        }
    }

    pub(crate) fn scan_statement(&mut self, statement: &'a Statement<'a>) {
        match statement {
            Statement::BlockStatement(block) => {
                for statement in &block.body {
                    self.scan_statement(statement);
                }
            }
            Statement::ExpressionStatement(statement) => {
                self.scan_expression(&statement.expression)
            }
            Statement::IfStatement(statement) => {
                self.scan_expression(&statement.test);
                self.scan_statement(&statement.consequent);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate);
                }
            }
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument);
                }
            }
            Statement::ThrowStatement(statement) => self.scan_expression(&statement.argument),
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test);
                self.scan_statement(&statement.body);
            }
            Statement::DoWhileStatement(statement) => {
                self.scan_statement(&statement.body);
                self.scan_expression(&statement.test);
            }
            Statement::ForStatement(statement) => {
                if let Some(init) = &statement.init {
                    self.scan_for_init(init);
                }
                if let Some(test) = &statement.test {
                    self.scan_expression(test);
                }
                if let Some(update) = &statement.update {
                    self.scan_expression(update);
                }
                self.scan_statement(&statement.body);
            }
            Statement::ForInStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right);
                self.scan_statement(&statement.body);
            }
            Statement::ForOfStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right);
                self.scan_statement(&statement.body);
            }
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test);
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
            Statement::LabeledStatement(statement) => self.scan_statement(&statement.body),
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Statement::FunctionDeclaration(function) => self.scan_function(function),
            Statement::ClassDeclaration(class) => self.scan_class(class),
            Statement::ExportNamedDeclaration(declaration) => {
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration);
                }
            }
            Statement::ExportDefaultDeclaration(declaration) => match &declaration.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                    self.scan_function(function);
                }
                ExportDefaultDeclarationKind::ClassDeclaration(class) => self.scan_class(class),
                _ => {
                    if let Some(expression) = declaration.declaration.as_expression() {
                        self.scan_expression(expression);
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
            Declaration::FunctionDeclaration(function) => self.scan_function(function),
            Declaration::ClassDeclaration(class) => self.scan_class(class),
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
                    self.scan_expression(expression);
                }
            }
        }
    }

    fn scan_for_left(&mut self, left: &'a ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration);
        }
    }

    fn scan_variable_declaration(&mut self, declaration: &'a VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            if let Some(init) = &declarator.init {
                self.scan_expression(init);
            }
        }
    }

    pub(crate) fn scan_function(&mut self, function: &'a Function<'a>) {
        for param in &function.params.items {
            if let Some(initializer) = &param.initializer {
                self.scan_expression(initializer);
            }
        }
        if let Some(body) = &function.body {
            self.scan_function_body(body);
        }
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>) {
        for statement in &body.statements {
            self.scan_statement(statement);
        }
    }

    pub(crate) fn scan_class(&mut self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class);
        }
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    for statement in &block.body {
                        self.scan_statement(statement);
                    }
                }
                ClassElement::MethodDefinition(method) => self.scan_function(&method.value),
                ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value);
                    }
                }
                ClassElement::TSIndexSignature(_) => {}
            }
        }
    }
}
