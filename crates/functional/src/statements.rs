//! Statement-level AST traversal for the functional scanner.

use oxc_ast::ast::*;

use crate::FunctionContext;
use crate::FunctionParamMeta;
use crate::helpers::is_mutable_type;
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_statement(
        &mut self,
        statement: &'a Statement<'a>,
        context: FunctionContext,
    ) {
        match statement {
            Statement::ExpressionStatement(statement) => {
                if !matches!(
                    statement.expression.get_inner_expression(),
                    Expression::StringLiteral(_) | Expression::YieldExpression(_)
                ) {
                    self.report(
                        "no-expression-statements",
                        "generic",
                        "Using expressions to cause side-effects not allowed.",
                        statement.span,
                    );
                }
                self.scan_expression(&statement.expression, context);
            }
            Statement::BlockStatement(block) => self.scan_statement_list(&block.body, context),
            Statement::IfStatement(statement) => {
                self.report(
                    "no-conditional-statements",
                    "unexpectedIf",
                    "Unexpected if, use a conditional expression (ternary operator) instead.",
                    statement.span,
                );
                self.scan_expression(&statement.test, context);
                self.scan_statement(&statement.consequent, context);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate, context);
                }
            }
            Statement::SwitchStatement(statement) => {
                self.report(
                    "no-conditional-statements",
                    "unexpectedSwitch",
                    "Unexpected switch, use a conditional expression instead.",
                    statement.span,
                );
                self.scan_expression(&statement.discriminant, context);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test, context);
                    }
                    self.scan_statement_list(&case.consequent, context);
                }
            }
            Statement::ForStatement(statement) => {
                self.report(
                    "no-loop-statements",
                    "generic",
                    "Unexpected loop, use map or reduce instead.",
                    statement.span,
                );
                if let Some(init) = &statement.init {
                    self.scan_for_init(init, context);
                }
                if let Some(test) = &statement.test {
                    self.scan_expression(test, context);
                }
                if let Some(update) = &statement.update {
                    self.scan_expression(update, context);
                }
                self.scan_statement(&statement.body, context);
            }
            Statement::ForInStatement(statement) => {
                self.report(
                    "no-loop-statements",
                    "generic",
                    "Unexpected loop, use map or reduce instead.",
                    statement.span,
                );
                self.scan_for_left(&statement.left, context);
                self.scan_expression(&statement.right, context);
                self.scan_statement(&statement.body, context);
            }
            Statement::ForOfStatement(statement) => {
                self.report(
                    "no-loop-statements",
                    "generic",
                    "Unexpected loop, use map or reduce instead.",
                    statement.span,
                );
                self.scan_for_left(&statement.left, context);
                self.scan_expression(&statement.right, context);
                self.scan_statement(&statement.body, context);
            }
            Statement::WhileStatement(statement) => {
                self.report(
                    "no-loop-statements",
                    "generic",
                    "Unexpected loop, use map or reduce instead.",
                    statement.span,
                );
                self.scan_expression(&statement.test, context);
                self.scan_statement(&statement.body, context);
            }
            Statement::DoWhileStatement(statement) => {
                self.report(
                    "no-loop-statements",
                    "generic",
                    "Unexpected loop, use map or reduce instead.",
                    statement.span,
                );
                self.scan_statement(&statement.body, context);
                self.scan_expression(&statement.test, context);
            }
            Statement::TryStatement(statement) => {
                if statement.handler.is_some() && !self.options.allow_try_catch {
                    self.report(
                        "no-try-statements",
                        "catch",
                        "Unexpected try-catch, this pattern is not functional.",
                        statement.span,
                    );
                } else if statement.finalizer.is_some() && !self.options.allow_try_finally {
                    self.report(
                        "no-try-statements",
                        "finally",
                        "Unexpected try-finally, this pattern is not functional.",
                        statement.span,
                    );
                }
                // A throw inside this try's block is caught when the try has a
                // catch handler, so it is not a promise rejection. The catch and
                // finally bodies are not protected by this try's own catch; they
                // inherit the enclosing context.
                let block_context = FunctionContext {
                    in_try_with_catch: context.in_try_with_catch || statement.handler.is_some(),
                    ..context
                };
                self.scan_statement_list(&statement.block.body, block_context);
                if let Some(handler) = &statement.handler {
                    self.scan_statement_list(&handler.body.body, context);
                }
                if let Some(finalizer) = &statement.finalizer {
                    self.scan_statement_list(&finalizer.body, context);
                }
            }
            Statement::ThrowStatement(statement) => {
                if !(self.options.allow_throw_to_reject_promises && context.in_async_function) {
                    self.report(
                        "no-throw-statements",
                        "generic",
                        "Unexpected throw, throwing exceptions is not functional.",
                        statement.span,
                    );
                }
                if context.in_async_function && !context.in_try_with_catch {
                    self.report(
                        "no-promise-reject",
                        "generic",
                        "Unexpected rejection, resolve an error instead.",
                        statement.span,
                    );
                }
                self.scan_expression(&statement.argument, context);
            }
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument, context);
                }
            }
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration, context, false);
            }
            Statement::FunctionDeclaration(function) => {
                let fn_name: Option<&'a str> = function.id.as_ref().map(|id| id.name.as_str());
                let meta = FunctionParamMeta {
                    name: fn_name,
                    ..FunctionParamMeta::default()
                };
                self.scan_function(function, meta);
            }
            Statement::ClassDeclaration(class) => self.scan_class(class, context),
            Statement::TSTypeAliasDeclaration(declaration) => {
                self.scan_type_alias_declaration(declaration);
            }
            Statement::TSInterfaceDeclaration(declaration) => {
                self.scan_interface_declaration(declaration);
            }
            Statement::ExportNamedDeclaration(declaration) => {
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration, context);
                }
            }
            Statement::ExportDefaultDeclaration(declaration) => match &declaration.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                    let fn_name: Option<&'a str> =
                        function.id.as_ref().map(|id| id.name.as_str());
                    let meta = FunctionParamMeta {
                        name: fn_name,
                        ..FunctionParamMeta::default()
                    };
                    self.scan_function(function, meta);
                }
                ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                    self.scan_class(class, context)
                }
                declaration => {
                    if let Some(expression) = declaration.as_expression() {
                        self.scan_expression(expression, context);
                    }
                }
            },
            _ => {}
        }
    }

    fn scan_declaration(&mut self, declaration: &'a Declaration<'a>, context: FunctionContext) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration, context, false);
            }
            Declaration::FunctionDeclaration(function) => {
                let fn_name: Option<&'a str> = function.id.as_ref().map(|id| id.name.as_str());
                let meta = FunctionParamMeta {
                    name: fn_name,
                    ..FunctionParamMeta::default()
                };
                self.scan_function(function, meta);
            }
            Declaration::ClassDeclaration(class) => self.scan_class(class, context),
            Declaration::TSTypeAliasDeclaration(declaration) => {
                self.scan_type_alias_declaration(declaration);
            }
            Declaration::TSInterfaceDeclaration(declaration) => {
                self.scan_interface_declaration(declaration);
            }
            _ => {}
        }
    }

    fn scan_for_init(&mut self, init: &'a ForStatementInit<'a>, context: FunctionContext) {
        match init {
            ForStatementInit::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration, context, true);
            }
            ForStatementInit::CallExpression(call) => self.scan_call_expression(call, context),
            ForStatementInit::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            ForStatementInit::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            ForStatementInit::AssignmentExpression(expression) => {
                self.scan_assignment_expression(expression, context);
            }
            ForStatementInit::SequenceExpression(expression) => {
                for expression in &expression.expressions {
                    self.scan_expression(expression, context);
                }
            }
            _ => {}
        }
    }

    fn scan_for_left(&mut self, left: &'a ForStatementLeft<'a>, context: FunctionContext) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration, context, true);
        }
    }

    fn declaration_identifiers_ignored(&self, declaration: &VariableDeclaration<'a>) -> bool {
        if self.ignore_identifier_regexes.is_empty() {
            return false;
        }
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return false;
            };
            if !self.matches_ignore_identifier(id.name.as_str()) {
                return false;
            }
        }
        true
    }

    fn scan_variable_declaration(
        &mut self,
        declaration: &'a VariableDeclaration<'a>,
        context: FunctionContext,
        in_for_init: bool,
    ) {
        let is_let = declaration.kind == VariableDeclarationKind::Let;
        let allowed_in_for_init = in_for_init && self.options.allow_let_in_for_loop_init;
        let allowed_in_function = self.options.allow_in_functions && context.in_function;
        let ignored = self.declaration_identifiers_ignored(declaration);
        if is_let && !allowed_in_for_init && !allowed_in_function && !ignored {
            self.report(
                "no-let",
                "generic",
                "Unexpected let, use const instead.",
                declaration.span,
            );
        }
        for declarator in &declaration.declarations {
            if let Some(type_annotation) = &declarator.type_annotation {
                self.scan_type(&type_annotation.type_annotation);
                if is_mutable_type(&type_annotation.type_annotation) {
                    self.report(
                        "prefer-immutable-types",
                        "parameter",
                        "Only readonly types allowed.",
                        type_annotation.span,
                    );
                }
            }
            if let Some(init) = &declarator.init {
                self.scan_expression(init, context);
            }
        }
    }
}
