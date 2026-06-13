//! AST traversal and dispatcher entry points for the e18e port.
//!
//! Each individual rule body lives in [`crate::rules`]; this module wires
//! traversal to the per-rule `check_*` methods through extra `impl Scanner`
//! blocks in each rule file.

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, AssignmentTarget, CallExpression, ChainElement, Class,
    ClassElement, Declaration, Expression, ForStatementInit, ForStatementLeft, Function,
    FunctionBody, NewExpression, ObjectPropertyKind, Program, RegExpFlags, Statement,
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{ExprContext, expression_body};
use crate::{Diagnostic, DiagnosticData, DiagnosticFix, E18eOptions, LineIndex};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) options: &'a E18eOptions,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 32]>,
    pub(crate) function_depth: usize,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan_program(&mut self, program: &'a Program<'a>) {
        for statement in &program.body {
            self.scan_statement(statement);
        }
    }

    pub(crate) fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span, None);
    }

    pub(crate) fn report_with_fix(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        replacement: impl Into<CompactString>,
    ) {
        self.report_with_data(
            rule_name,
            message_id,
            DiagnosticData::default(),
            span,
            Some(DiagnosticFix {
                start: span.start,
                end: span.end,
                replacement: replacement.into(),
            }),
        );
    }

    pub(crate) fn report_with_data(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: DiagnosticData,
        span: Span,
        fix: Option<DiagnosticFix>,
    ) {
        if !self.options.has_rule(rule_name) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix,
        });
    }

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

    pub(crate) fn scan_expression(&mut self, expression: &'a Expression<'a>, context: ExprContext) {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => {
                self.check_call_expression(call, context);
                self.scan_expression(&call.callee, ExprContext::Callee);
                for argument in &call.arguments {
                    self.scan_argument(argument);
                }
            }
            Expression::NewExpression(new_expression) => {
                self.check_new_expression(new_expression);
                self.scan_expression(&new_expression.callee, ExprContext::Callee);
                for argument in &new_expression.arguments {
                    self.scan_argument(argument);
                }
            }
            Expression::StaticMemberExpression(member) => {
                self.check_static_member_expression(member, context);
                self.scan_expression(&member.object, ExprContext::MemberObject);
            }
            Expression::ComputedMemberExpression(member) => {
                self.check_computed_member_expression(member);
                self.scan_expression(&member.object, ExprContext::MemberObject);
                self.scan_expression(&member.expression, ExprContext::Other);
            }
            Expression::AssignmentExpression(assignment) => {
                self.scan_assignment_target(&assignment.left);
                self.scan_expression(&assignment.right, ExprContext::Other);
            }
            Expression::BinaryExpression(binary) => {
                self.check_binary_expression(binary);
                self.scan_expression(&binary.left, ExprContext::Other);
                self.scan_expression(&binary.right, ExprContext::Other);
            }
            Expression::LogicalExpression(logical) => {
                self.scan_expression(&logical.left, ExprContext::Boolean);
                self.scan_expression(&logical.right, ExprContext::Boolean);
            }
            Expression::ConditionalExpression(conditional) => {
                self.check_prefer_nullish_conditional(conditional);
                self.scan_expression(&conditional.test, ExprContext::Boolean);
                self.scan_expression(&conditional.consequent, ExprContext::Other);
                self.scan_expression(&conditional.alternate, ExprContext::Other);
            }
            Expression::UnaryExpression(unary) => {
                self.check_unary_expression(unary, context);
                self.scan_expression(&unary.argument, context);
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    self.scan_array_element(element);
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            self.scan_expression(&property.value, ExprContext::Other);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(&spread.argument, ExprContext::Other);
                        }
                    }
                }
            }
            Expression::ArrowFunctionExpression(function) => {
                self.function_depth += 1;
                if function.expression {
                    if let Some(expression) = expression_body(&function.body) {
                        self.scan_expression(expression, ExprContext::Return);
                    }
                } else {
                    self.scan_function_body(&function.body);
                }
                self.function_depth -= 1;
            }
            Expression::FunctionExpression(function) => self.scan_function(function),
            Expression::ClassExpression(class) => self.scan_class(class),
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression, ExprContext::Other);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.scan_expression(&tagged.tag, ExprContext::Callee);
                for expression in &tagged.quasi.expressions {
                    self.scan_expression(expression, ExprContext::Other);
                }
            }
            Expression::SequenceExpression(sequence) => {
                for expression in &sequence.expressions {
                    self.scan_expression(expression, ExprContext::Other);
                }
            }
            Expression::AwaitExpression(await_expression) => {
                self.scan_expression(&await_expression.argument, ExprContext::Other);
            }
            Expression::YieldExpression(yield_expression) => {
                if let Some(argument) = &yield_expression.argument {
                    self.scan_expression(argument, ExprContext::Other);
                }
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    self.check_call_expression(call, context);
                    self.scan_expression(&call.callee, ExprContext::Callee);
                    for argument in &call.arguments {
                        self.scan_argument(argument);
                    }
                }
                ChainElement::TSNonNullExpression(expression) => {
                    self.scan_expression(&expression.expression, context);
                }
                ChainElement::StaticMemberExpression(member) => {
                    self.check_static_member_expression(member, context);
                    self.scan_expression(&member.object, ExprContext::MemberObject);
                }
                ChainElement::ComputedMemberExpression(member) => {
                    self.check_computed_member_expression(member);
                    self.scan_expression(&member.object, ExprContext::MemberObject);
                    self.scan_expression(&member.expression, ExprContext::Other);
                }
                ChainElement::PrivateFieldExpression(member) => {
                    self.scan_expression(&member.object, ExprContext::MemberObject);
                }
            },
            Expression::ImportExpression(import) => {
                if let Expression::StringLiteral(source) = import.source.get_inner_expression() {
                    self.check_ban_dependency_source(source.value.as_str(), source.span);
                }
                self.scan_expression(&import.source, ExprContext::Other);
                if let Some(options) = &import.options {
                    self.scan_expression(options, ExprContext::Other);
                }
            }
            Expression::RegExpLiteral(literal) => {
                if self.function_depth > 0
                    && !literal.regex.flags.contains(RegExpFlags::G)
                    && !literal.regex.flags.contains(RegExpFlags::Y)
                {
                    self.report("prefer-static-regex", "preferStatic", literal.span);
                }
            }
            Expression::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            Expression::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSInstantiationExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            Expression::ParenthesizedExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            _ => {}
        }
    }

    pub(crate) fn scan_argument(&mut self, argument: &'a Argument<'a>) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression, ExprContext::Other);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument, ExprContext::Other);
        }
    }

    pub(crate) fn scan_array_element(&mut self, element: &'a ArrayExpressionElement<'a>) {
        if let Some(expression) = element.as_expression() {
            self.scan_expression(expression, ExprContext::Other);
        } else if let ArrayExpressionElement::SpreadElement(spread) = element {
            self.scan_expression(&spread.argument, ExprContext::Other);
        }
    }

    pub(crate) fn scan_assignment_target(&mut self, target: &'a AssignmentTarget<'a>) {
        match target {
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ExprContext::MemberObject);
                self.scan_expression(&member.expression, ExprContext::Other);
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                self.scan_expression(&member.object, ExprContext::MemberObject);
            }
            _ => {}
        }
    }

    pub(crate) fn check_call_expression(
        &mut self,
        call: &'a CallExpression<'a>,
        context: ExprContext,
    ) {
        self.check_ban_dependency_require(call);
        self.check_prefer_exponentiation(call);
        self.check_prefer_object_has_own(call);
        self.check_prefer_array_from_map(call);
        self.check_prefer_array_fill(call);
        self.check_prefer_spread_syntax(call);
        self.check_prefer_copy_method(call);
        self.check_prefer_date_now_call(call);
        self.check_prefer_regex_test(call, context);
        self.check_prefer_array_some_call(call, context);
        self.check_prefer_static_regex_call(call);
        self.check_prefer_inline_equality(call);
        self.check_prefer_string_from_char_code(call);
        self.check_prefer_timer_args(call);
        self.check_prefer_includes_over_regex_test(call);
        self.check_no_spread_in_reduce(call);
        self.check_prefer_static_collator(call);
    }

    pub(crate) fn check_new_expression(&mut self, new_expression: &'a NewExpression<'a>) {
        self.check_prefer_static_regex_new(new_expression);
        self.check_prefer_date_now_new(new_expression);
    }

    pub(crate) fn check_static_member_expression(
        &mut self,
        member: &'a oxc_ast::ast::StaticMemberExpression<'a>,
        context: ExprContext,
    ) {
        self.check_filter_length_member(member, context);
    }

    pub(crate) fn check_computed_member_expression(
        &mut self,
        member: &'a oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        self.check_prefer_array_at(member);
    }

    pub(crate) fn check_binary_expression(
        &mut self,
        binary: &'a oxc_ast::ast::BinaryExpression<'a>,
    ) {
        self.check_prefer_includes_binary(binary);
        self.check_no_indexof_equality(binary);
        self.check_prefer_array_some_binary(binary);
    }

    pub(crate) fn check_unary_expression(
        &mut self,
        unary: &'a oxc_ast::ast::UnaryExpression<'a>,
        context: ExprContext,
    ) {
        self.check_prefer_includes_unary(unary);
        self.check_prefer_array_some_unary(unary);
        self.check_prefer_date_now_unary(unary);
        self.check_no_delete_property(unary, context);
    }
    pub(crate) fn text(&self, span: Span) -> &'a str {
        &self.source_text[span.start as usize..span.end as usize]
    }
}
