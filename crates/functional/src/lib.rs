#![doc = "Rust implementation of eslint-plugin-functional rule logic."]
#![allow(
    clippy::disallowed_types,
    reason = "The first native functional port builds NAPI-facing diagnostics and small AST worklists; hot string data is compacted."
)]

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionExpression, AssignmentTarget, BindingPattern,
    CallExpression, ChainElement, Class, ClassElement, ComputedMemberExpression, Declaration,
    ExportDefaultDeclarationKind, Expression, ForStatementInit, ForStatementLeft, FormalParameters,
    Function, FunctionBody, NewExpression, ObjectPropertyKind, PropertyKey, Statement,
    StaticMemberExpression, TSInterfaceDeclaration, TSSignature, TSType, TSTypeAliasDeclaration,
    TSTypeName, TSTypeOperator, TSTypeOperatorOperator, TSTypeReference, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub const RULE_NAMES: [&str; 20] = [
    "functional-parameters",
    "immutable-data",
    "no-class-inheritance",
    "no-classes",
    "no-conditional-statements",
    "no-expression-statements",
    "no-let",
    "no-loop-statements",
    "no-mixed-types",
    "no-promise-reject",
    "no-return-void",
    "no-this-expressions",
    "no-throw-statements",
    "no-try-statements",
    "prefer-immutable-types",
    "prefer-property-signatures",
    "prefer-readonly-type",
    "prefer-tacit",
    "readonly-type",
    "type-declaration-immutability",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message: CompactString,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionalOptions {
    pub rule_names: SmallVec<[CompactString; 20]>,
    pub allow_rest_parameter: bool,
    pub allow_arguments_keyword: bool,
    pub allow_let_in_for_loop_init: bool,
    pub allow_throw_to_reject_promises: bool,
    pub allow_try_catch: bool,
    pub allow_try_finally: bool,
    pub readonly_type_mode: CompactString,
}

impl Default for FunctionalOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            allow_rest_parameter: false,
            allow_arguments_keyword: false,
            allow_let_in_for_loop_init: false,
            allow_throw_to_reject_promises: false,
            allow_try_catch: false,
            allow_try_finally: false,
            readonly_type_mode: "generic".into(),
        }
    }
}

impl FunctionalOptions {
    fn has_rule(&self, rule_name: &str) -> bool {
        self.rule_names.iter().any(|name| name == rule_name)
    }
}

struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    fn new(source_text: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn loc_for_span(&self, source_text: &str, span: Span) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source_text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        let line_index = line_index.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = source_text[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line_index + 1) as u32, column as u32)
    }
}

#[derive(Clone, Copy)]
struct FunctionContext {
    in_async_function: bool,
}

pub fn implemented_functional_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_functional(
    source_text: &str,
    filename: &str,
    options: &FunctionalOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        options,
    };
    scanner.scan_statement_list(
        &parser_return.program.body,
        FunctionContext {
            in_async_function: false,
        },
    );
    scanner.diagnostics
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 32]>,
    options: &'a FunctionalOptions,
}

impl<'a> Scanner<'a> {
    fn report(&mut self, rule_name: &'static str, message: &'static str, span: Span) {
        if self.options.has_rule(rule_name) {
            self.diagnostics.push(Diagnostic {
                rule_name,
                message: message.into(),
                loc: self.line_index.loc_for_span(self.source_text, span),
            });
        }
    }

    fn scan_statement_list(&mut self, statements: &'a [Statement<'a>], context: FunctionContext) {
        for statement in statements {
            self.scan_statement(statement, context);
        }
    }

    fn scan_statement(&mut self, statement: &'a Statement<'a>, context: FunctionContext) {
        match statement {
            Statement::ExpressionStatement(statement) => {
                if !matches!(
                    statement.expression.get_inner_expression(),
                    Expression::StringLiteral(_) | Expression::YieldExpression(_)
                ) {
                    self.report(
                        "no-expression-statements",
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
                    "Unexpected loop, use map or reduce instead.",
                    statement.span,
                );
                self.scan_expression(&statement.test, context);
                self.scan_statement(&statement.body, context);
            }
            Statement::DoWhileStatement(statement) => {
                self.report(
                    "no-loop-statements",
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
                        "Unexpected try-catch, this pattern is not functional.",
                        statement.span,
                    );
                } else if statement.finalizer.is_some() && !self.options.allow_try_finally {
                    self.report(
                        "no-try-statements",
                        "Unexpected try-finally, this pattern is not functional.",
                        statement.span,
                    );
                }
                self.scan_statement_list(&statement.block.body, context);
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
                        "Unexpected throw, throwing exceptions is not functional.",
                        statement.span,
                    );
                }
                if context.in_async_function {
                    self.report(
                        "no-promise-reject",
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
            Statement::FunctionDeclaration(function) => self.scan_function(function),
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
                    self.scan_function(function)
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
            Declaration::FunctionDeclaration(function) => self.scan_function(function),
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

    fn scan_variable_declaration(
        &mut self,
        declaration: &'a VariableDeclaration<'a>,
        context: FunctionContext,
        in_for_init: bool,
    ) {
        if declaration.kind == VariableDeclarationKind::Let
            && !(in_for_init && self.options.allow_let_in_for_loop_init)
        {
            self.report(
                "no-let",
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

    fn scan_expression(&mut self, expression: &'a Expression<'a>, context: FunctionContext) {
        match expression.get_inner_expression() {
            Expression::Identifier(identifier) => {
                if identifier.name == "arguments" && !self.options.allow_arguments_keyword {
                    self.report(
                        "functional-parameters",
                        "Unexpected use of `arguments`. Use regular function arguments instead.",
                        identifier.span,
                    );
                }
            }
            Expression::ThisExpression(expression) => {
                self.report(
                    "no-this-expressions",
                    "Unexpected this, use functions not classes.",
                    expression.span,
                );
            }
            Expression::CallExpression(call) => self.scan_call_expression(call, context),
            Expression::NewExpression(expression) => self.scan_new_expression(expression, context),
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => self.scan_call_expression(call, context),
                ChainElement::StaticMemberExpression(member) => {
                    self.scan_static_member_expression(member, context);
                }
                _ => {}
            },
            Expression::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            Expression::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            Expression::AssignmentExpression(expression) => {
                self.scan_assignment_expression(expression, context);
            }
            Expression::UpdateExpression(expression) => {
                self.report(
                    "immutable-data",
                    "Modifying an existing object/array is not allowed.",
                    expression.span,
                );
            }
            Expression::ArrowFunctionExpression(function) => self.scan_arrow_function(function),
            Expression::FunctionExpression(function) => self.scan_function(function),
            Expression::ClassExpression(class) => self.scan_class(class, context),
            Expression::ObjectExpression(expression) => {
                for property in &expression.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed {
                                self.scan_property_key(&property.key, context);
                            }
                            self.scan_expression(&property.value, context);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(&spread.argument, context);
                        }
                    }
                }
            }
            Expression::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, context);
                }
            }
            Expression::AwaitExpression(expression) => {
                self.scan_expression(&expression.argument, context);
            }
            Expression::UnaryExpression(expression) => {
                self.scan_expression(&expression.argument, context);
            }
            Expression::BinaryExpression(expression) => {
                self.scan_expression(&expression.left, context);
                self.scan_expression(&expression.right, context);
            }
            Expression::LogicalExpression(expression) => {
                self.scan_expression(&expression.left, context);
                self.scan_expression(&expression.right, context);
            }
            Expression::ConditionalExpression(expression) => {
                self.scan_expression(&expression.test, context);
                self.scan_expression(&expression.consequent, context);
                self.scan_expression(&expression.alternate, context);
            }
            Expression::SequenceExpression(expression) => {
                for expression in &expression.expressions {
                    self.scan_expression(expression, context);
                }
            }
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression, context);
                }
            }
            Expression::TaggedTemplateExpression(expression) => {
                self.scan_expression(&expression.tag, context);
                for expression in &expression.quasi.expressions {
                    self.scan_expression(expression, context);
                }
            }
            Expression::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            _ => {}
        }
    }

    fn scan_array_element(
        &mut self,
        element: &'a ArrayExpressionElement<'a>,
        context: FunctionContext,
    ) {
        match element {
            ArrayExpressionElement::SpreadElement(spread) => {
                self.scan_expression(&spread.argument, context)
            }
            ArrayExpressionElement::CallExpression(call) => {
                self.scan_call_expression(call, context)
            }
            ArrayExpressionElement::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            ArrayExpressionElement::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            ArrayExpressionElement::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function)
            }
            ArrayExpressionElement::FunctionExpression(function) => self.scan_function(function),
            ArrayExpressionElement::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, context);
                }
            }
            ArrayExpressionElement::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property {
                        self.scan_expression(&property.value, context);
                    }
                }
            }
            _ => {}
        }
    }

    fn scan_argument(&mut self, argument: &'a Argument<'a>, context: FunctionContext) {
        match argument {
            Argument::SpreadElement(spread) => self.scan_expression(&spread.argument, context),
            Argument::CallExpression(call) => self.scan_call_expression(call, context),
            Argument::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            Argument::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            Argument::ArrowFunctionExpression(function) => self.scan_arrow_function(function),
            Argument::FunctionExpression(function) => self.scan_function(function),
            Argument::ClassExpression(class) => self.scan_class(class, context),
            Argument::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, context);
                }
            }
            Argument::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property {
                        self.scan_expression(&property.value, context);
                    }
                }
            }
            _ => {}
        }
    }

    fn scan_property_key(&mut self, key: &'a PropertyKey<'a>, context: FunctionContext) {
        match key {
            PropertyKey::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            PropertyKey::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            PropertyKey::CallExpression(call) => self.scan_call_expression(call, context),
            _ => {}
        }
    }

    fn scan_assignment_expression(
        &mut self,
        expression: &'a oxc_ast::ast::AssignmentExpression<'a>,
        context: FunctionContext,
    ) {
        if assignment_target_is_member(&expression.left) {
            self.report(
                "immutable-data",
                "Modifying an existing object/array is not allowed.",
                expression.span,
            );
        }
        self.scan_assignment_target(&expression.left, context);
        self.scan_expression(&expression.right, context);
    }

    fn scan_assignment_target(
        &mut self,
        target: &'a AssignmentTarget<'a>,
        context: FunctionContext,
    ) {
        match target {
            AssignmentTarget::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            AssignmentTarget::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object, context);
            }
            AssignmentTarget::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            AssignmentTarget::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            AssignmentTarget::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            AssignmentTarget::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            _ => {}
        }
    }

    fn scan_static_member_expression(
        &mut self,
        member: &'a StaticMemberExpression<'a>,
        context: FunctionContext,
    ) {
        self.scan_expression(&member.object, context);
    }

    fn scan_computed_member_expression(
        &mut self,
        member: &'a ComputedMemberExpression<'a>,
        context: FunctionContext,
    ) {
        self.scan_expression(&member.object, context);
        self.scan_expression(&member.expression, context);
    }

    fn scan_call_expression(&mut self, call: &'a CallExpression<'a>, context: FunctionContext) {
        if is_static_call(call, "Promise", "reject") {
            self.report(
                "no-promise-reject",
                "Unexpected rejection, resolve an error instead.",
                call.span,
            );
        }
        if is_mutating_call(call) {
            self.report(
                "immutable-data",
                "Modifying an existing object/array is not allowed.",
                call.span,
            );
        }
        self.scan_expression(&call.callee, context);
        for argument in &call.arguments {
            self.scan_argument(argument, context);
        }
    }

    fn scan_new_expression(&mut self, expression: &'a NewExpression<'a>, context: FunctionContext) {
        if is_identifier_expression(&expression.callee, "Promise")
            && expression.arguments.len() >= 2
        {
            self.report(
                "no-promise-reject",
                "Unexpected rejection, resolve an error instead.",
                expression.span,
            );
        }
        self.scan_expression(&expression.callee, context);
        for argument in &expression.arguments {
            self.scan_argument(argument, context);
        }
    }

    fn scan_function(&mut self, function: &'a Function<'a>) {
        self.scan_function_parameters(&function.params, function.span);
        if let Some(return_type) = &function.return_type {
            self.scan_return_type(return_type);
        }
        let context = FunctionContext {
            in_async_function: function.r#async,
        };
        if let Some(body) = &function.body {
            self.scan_function_body(body, context);
        }
    }

    fn scan_arrow_function(&mut self, function: &'a ArrowFunctionExpression<'a>) {
        self.scan_function_parameters(&function.params, function.span);
        self.check_prefer_tacit(function);
        if let Some(return_type) = &function.return_type {
            self.scan_return_type(return_type);
        }
        let context = FunctionContext {
            in_async_function: function.r#async,
        };
        self.scan_function_body(&function.body, context);
    }

    fn scan_function_parameters(&mut self, params: &'a FormalParameters<'a>, span: Span) {
        if params.items.is_empty() && params.rest.is_none() {
            self.report(
                "functional-parameters",
                "Functions must have at least one parameter.",
                span,
            );
        }
        if let Some(rest) = &params.rest
            && !self.options.allow_rest_parameter
        {
            self.report(
                "functional-parameters",
                "Unexpected rest parameter. Use a regular parameter of type array instead.",
                rest.span,
            );
        }
        for param in &params.items {
            if let Some(type_annotation) = &param.type_annotation {
                self.scan_type(&type_annotation.type_annotation);
                if is_mutable_type(&type_annotation.type_annotation) {
                    self.report(
                        "prefer-immutable-types",
                        "Only readonly types allowed.",
                        type_annotation.span,
                    );
                }
            }
            if param.readonly {
                self.report(
                    "readonly-type",
                    "Readonly type using 'readonly' keyword is forbidden. Use 'Readonly<T>' instead.",
                    param.span,
                );
            }
            if let Some(init) = &param.initializer {
                self.scan_expression(
                    init,
                    FunctionContext {
                        in_async_function: false,
                    },
                );
            }
        }
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>, context: FunctionContext) {
        self.scan_statement_list(&body.statements, context);
    }

    fn scan_return_type(&mut self, return_type: &'a oxc_ast::ast::TSTypeAnnotation<'a>) {
        match &return_type.type_annotation {
            TSType::TSVoidKeyword(_) => {
                self.report(
                    "no-return-void",
                    "Function must return a value.",
                    return_type.span,
                );
            }
            TSType::TSNullKeyword(_) => {
                self.report(
                    "no-return-void",
                    "Function must return a value.",
                    return_type.span,
                );
            }
            TSType::TSUndefinedKeyword(_) => {
                self.report(
                    "no-return-void",
                    "Function must return a value.",
                    return_type.span,
                );
            }
            _ => {}
        }
        self.scan_type(&return_type.type_annotation);
    }

    fn scan_class(&mut self, class: &'a Class<'a>, context: FunctionContext) {
        self.report(
            "no-classes",
            "Unexpected class, use functions not classes.",
            class.span,
        );
        if class.super_class.is_some() {
            self.report(
                "no-class-inheritance",
                "Unexpected class inheritance.",
                class.span,
            );
        }
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class, context);
        }
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => self.scan_statement_list(&block.body, context),
                ClassElement::MethodDefinition(method) => self.scan_function(&method.value),
                ClassElement::PropertyDefinition(property) => {
                    if let Some(type_annotation) = &property.type_annotation {
                        self.scan_type(&type_annotation.type_annotation);
                        if is_mutable_type(&type_annotation.type_annotation) && !property.readonly {
                            self.report(
                                "prefer-readonly-type",
                                "A readonly modifier is required.",
                                property.span,
                            );
                        }
                    }
                    if let Some(value) = &property.value {
                        self.scan_expression(value, context);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, context);
                    }
                }
                ClassElement::TSIndexSignature(signature) => {
                    if !signature.readonly {
                        self.report(
                            "prefer-readonly-type",
                            "A readonly modifier is required.",
                            signature.span,
                        );
                    }
                    self.scan_type(&signature.type_annotation.type_annotation);
                }
            }
        }
    }

    fn check_prefer_tacit(&mut self, function: &'a ArrowFunctionExpression<'a>) {
        if function.params.rest.is_some() || function.params.items.is_empty() {
            return;
        }
        let Some(call) = single_returned_call(&function.body) else {
            return;
        };
        if !call_arguments_match_params(call, &function.params) {
            return;
        }
        self.report(
            "prefer-tacit",
            "Potentially unnecessary function wrapper.",
            function.span,
        );
    }

    fn scan_type_alias_declaration(&mut self, declaration: &'a TSTypeAliasDeclaration<'a>) {
        if let TSType::TSTypeLiteral(type_literal) = &declaration.type_annotation
            && has_mixed_signatures(&type_literal.members)
        {
            self.report(
                "no-mixed-types",
                "Only the same kind of members allowed in types.",
                declaration.span,
            );
        }
        if is_mutable_type(&declaration.type_annotation) {
            self.report(
                "type-declaration-immutability",
                "This type declaration contains mutable members.",
                declaration.span,
            );
        }
        self.scan_type(&declaration.type_annotation);
    }

    fn scan_interface_declaration(&mut self, declaration: &'a TSInterfaceDeclaration<'a>) {
        if has_mixed_signatures(&declaration.body.body) {
            self.report(
                "no-mixed-types",
                "Only the same kind of members allowed in types.",
                declaration.span,
            );
        }
        if interface_has_mutable_members(&declaration.body.body) {
            self.report(
                "type-declaration-immutability",
                "This type declaration contains mutable members.",
                declaration.span,
            );
        }
        for signature in &declaration.body.body {
            self.scan_signature(signature);
        }
    }

    fn scan_signature(&mut self, signature: &'a TSSignature<'a>) {
        match signature {
            TSSignature::TSMethodSignature(method) => {
                self.report(
                    "prefer-property-signatures",
                    "Use a property signature instead of a method signature",
                    method.span,
                );
                if let Some(return_type) = &method.return_type {
                    self.scan_return_type(return_type);
                }
            }
            TSSignature::TSPropertySignature(property) => {
                if property.readonly && self.options.readonly_type_mode == "generic" {
                    self.report(
                        "readonly-type",
                        "Readonly type using 'readonly' keyword is forbidden. Use 'Readonly<T>' instead.",
                        property.span,
                    );
                }
                if !property.readonly {
                    self.report(
                        "prefer-readonly-type",
                        "A readonly modifier is required.",
                        property.span,
                    );
                }
                if let Some(type_annotation) = &property.type_annotation {
                    self.scan_type(&type_annotation.type_annotation);
                }
            }
            TSSignature::TSIndexSignature(signature) => {
                if !signature.readonly {
                    self.report(
                        "prefer-readonly-type",
                        "A readonly modifier is required.",
                        signature.span,
                    );
                }
                self.scan_type(&signature.type_annotation.type_annotation);
            }
            TSSignature::TSCallSignatureDeclaration(signature) => {
                if let Some(return_type) = &signature.return_type {
                    self.scan_return_type(return_type);
                }
            }
            TSSignature::TSConstructSignatureDeclaration(signature) => {
                if let Some(return_type) = &signature.return_type {
                    self.scan_return_type(return_type);
                }
            }
        }
    }

    fn scan_type(&mut self, ty: &'a TSType<'a>) {
        match ty {
            TSType::TSArrayType(array) => {
                self.report(
                    "prefer-readonly-type",
                    "Only readonly arrays allowed.",
                    array.span,
                );
                self.report(
                    "prefer-immutable-types",
                    "Only readonly types allowed.",
                    array.span,
                );
                self.scan_type(&array.element_type);
            }
            TSType::TSTupleType(tuple) => {
                self.report(
                    "prefer-readonly-type",
                    "Only readonly tuples allowed.",
                    tuple.span,
                );
            }
            TSType::TSTypeReference(reference) => {
                if type_reference_name(reference).is_some_and(is_mutable_collection_name) {
                    self.report(
                        "prefer-readonly-type",
                        "Only readonly types allowed.",
                        reference.span,
                    );
                    self.report(
                        "prefer-immutable-types",
                        "Only readonly types allowed.",
                        reference.span,
                    );
                }
                if let Some(arguments) = &reference.type_arguments {
                    for ty in &arguments.params {
                        self.scan_type(ty);
                    }
                }
            }
            TSType::TSTypeOperatorType(operator) => {
                self.scan_type_operator(operator);
            }
            TSType::TSTypeLiteral(literal) => {
                if has_mixed_signatures(&literal.members) {
                    self.report(
                        "no-mixed-types",
                        "Only the same kind of members allowed in types.",
                        literal.span,
                    );
                }
                for signature in &literal.members {
                    self.scan_signature(signature);
                }
            }
            TSType::TSUnionType(union) => {
                for ty in &union.types {
                    self.scan_type(ty);
                }
            }
            TSType::TSIntersectionType(intersection) => {
                for ty in &intersection.types {
                    self.scan_type(ty);
                }
            }
            TSType::TSParenthesizedType(parenthesized) => {
                self.scan_type(&parenthesized.type_annotation);
            }
            TSType::TSFunctionType(function) => {
                self.scan_return_type(&function.return_type);
            }
            _ => {}
        }
    }

    fn scan_type_operator(&mut self, operator: &'a TSTypeOperator<'a>) {
        if operator.operator == TSTypeOperatorOperator::Readonly {
            if self.options.readonly_type_mode == "keyword" {
                self.report(
                    "readonly-type",
                    "Readonly type using 'Readonly<T>' is forbidden. Use 'readonly' keyword instead.",
                    operator.span,
                );
            }
        } else {
            self.scan_type(&operator.type_annotation);
        }
    }
}

fn assignment_target_is_member(target: &AssignmentTarget<'_>) -> bool {
    matches!(
        target,
        AssignmentTarget::StaticMemberExpression(_)
            | AssignmentTarget::ComputedMemberExpression(_)
            | AssignmentTarget::PrivateFieldExpression(_)
    )
}

fn is_identifier_expression(expression: &Expression<'_>, name: &str) -> bool {
    matches!(expression.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == name)
}

fn is_static_call(call: &CallExpression<'_>, object_name: &str, method_name: &str) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    member.property.name == method_name && is_identifier_expression(&member.object, object_name)
}

fn is_mutating_call(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    let method = member.property.name.as_str();
    matches!(
        method,
        "copyWithin"
            | "fill"
            | "pop"
            | "push"
            | "reverse"
            | "shift"
            | "sort"
            | "splice"
            | "unshift"
            | "clear"
            | "delete"
            | "set"
            | "add"
            | "assign"
            | "defineProperties"
            | "defineProperty"
            | "setPrototypeOf"
    )
}

fn type_reference_name<'a>(reference: &'a TSTypeReference<'a>) -> Option<&'a str> {
    match &reference.type_name {
        TSTypeName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        TSTypeName::QualifiedName(qualified) => Some(qualified.right.name.as_str()),
        TSTypeName::ThisExpression(_) => None,
    }
}

fn is_mutable_collection_name(name: &str) -> bool {
    matches!(name, "Array" | "Map" | "Set" | "WeakMap" | "WeakSet")
}

fn is_mutable_type(ty: &TSType<'_>) -> bool {
    match ty {
        TSType::TSArrayType(_) | TSType::TSTupleType(_) => true,
        TSType::TSTypeReference(reference) => {
            type_reference_name(reference).is_some_and(is_mutable_collection_name)
        }
        TSType::TSTypeLiteral(literal) => interface_has_mutable_members(&literal.members),
        TSType::TSUnionType(union) => union.types.iter().any(is_mutable_type),
        TSType::TSIntersectionType(intersection) => intersection.types.iter().any(is_mutable_type),
        TSType::TSParenthesizedType(parenthesized) => {
            is_mutable_type(&parenthesized.type_annotation)
        }
        TSType::TSTypeOperatorType(operator) => {
            operator.operator != TSTypeOperatorOperator::Readonly
                && is_mutable_type(&operator.type_annotation)
        }
        _ => false,
    }
}

fn signature_is_function_like(signature: &TSSignature<'_>) -> bool {
    match signature {
        TSSignature::TSMethodSignature(_)
        | TSSignature::TSCallSignatureDeclaration(_)
        | TSSignature::TSConstructSignatureDeclaration(_) => true,
        TSSignature::TSPropertySignature(property) => {
            property.type_annotation.as_ref().is_some_and(|annotation| {
                matches!(annotation.type_annotation, TSType::TSFunctionType(_))
            })
        }
        TSSignature::TSIndexSignature(_) => false,
    }
}

fn has_mixed_signatures(signatures: &[TSSignature<'_>]) -> bool {
    if signatures.len() < 2 {
        return false;
    }
    let first = signature_is_function_like(&signatures[0]);
    signatures
        .iter()
        .skip(1)
        .any(|signature| signature_is_function_like(signature) != first)
}

fn interface_has_mutable_members(signatures: &[TSSignature<'_>]) -> bool {
    signatures.iter().any(|signature| match signature {
        TSSignature::TSPropertySignature(property) => {
            !property.readonly
                || property
                    .type_annotation
                    .as_ref()
                    .is_some_and(|annotation| is_mutable_type(&annotation.type_annotation))
        }
        TSSignature::TSIndexSignature(signature) => {
            !signature.readonly || is_mutable_type(&signature.type_annotation.type_annotation)
        }
        _ => false,
    })
}

fn single_returned_call<'a>(body: &'a FunctionBody<'a>) -> Option<&'a CallExpression<'a>> {
    if body.statements.len() != 1 {
        return None;
    }
    match &body.statements[0] {
        Statement::ExpressionStatement(statement) => {
            match statement.expression.get_inner_expression() {
                Expression::CallExpression(call) => Some(&**call),
                _ => None,
            }
        }
        Statement::ReturnStatement(statement) => statement.argument.as_ref().and_then(|argument| {
            if let Expression::CallExpression(call) = argument.get_inner_expression() {
                Some(&**call)
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn call_arguments_match_params(call: &CallExpression<'_>, params: &FormalParameters<'_>) -> bool {
    if call.arguments.len() != params.items.len() {
        return false;
    }
    call.arguments
        .iter()
        .zip(params.items.iter())
        .all(|(argument, param)| {
            let Argument::Identifier(argument) = argument else {
                return false;
            };
            let BindingPattern::BindingIdentifier(parameter) = &param.pattern else {
                return false;
            };
            argument.name == parameter.name
        })
}

#[cfg(test)]
mod tests {
    use super::{FunctionalOptions, implemented_functional_rule_names, scan_functional};

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(implemented_functional_rule_names().len(), 20);
        assert!(implemented_functional_rule_names().contains(&"no-let"));
        assert!(implemented_functional_rule_names().contains(&"prefer-readonly-type"));
    }

    #[test]
    fn scans_core_syntax_rules() {
        let diagnostics = scan_functional(
            r#"
let value = 1;
class Derived extends Base { method() { this.x = 1; } }
if (value) { value += 1; }
for (let i = 0; i < 1; i++) {}
try { throw new Error('x'); } catch (err) {}
Promise.reject(err);
const f = () => effect();
"#,
            "fixture.ts",
            &FunctionalOptions::default(),
        );
        let rules: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();
        assert!(rules.contains(&"no-let"));
        assert!(rules.contains(&"no-classes"));
        assert!(rules.contains(&"no-class-inheritance"));
        assert!(rules.contains(&"no-conditional-statements"));
        assert!(rules.contains(&"no-loop-statements"));
        assert!(rules.contains(&"no-try-statements"));
        assert!(rules.contains(&"no-throw-statements"));
        assert!(rules.contains(&"no-promise-reject"));
        assert!(rules.contains(&"no-this-expressions"));
        assert!(rules.contains(&"functional-parameters"));
    }

    #[test]
    fn scans_type_rules() {
        let diagnostics = scan_functional(
            r#"
interface Mixed {
  readonly items: string[];
  run(): void;
}
type Bag = { value: Array<string> };
const takes = (items: string[]): void => {};
"#,
            "fixture.ts",
            &FunctionalOptions::default(),
        );
        let rules: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();
        assert!(rules.contains(&"no-mixed-types"));
        assert!(rules.contains(&"prefer-property-signatures"));
        assert!(rules.contains(&"prefer-readonly-type"));
        assert!(rules.contains(&"prefer-immutable-types"));
        assert!(rules.contains(&"readonly-type"));
        assert!(rules.contains(&"type-declaration-immutability"));
        assert!(rules.contains(&"no-return-void"));
    }

    #[test]
    fn honors_core_options() {
        let mut options = FunctionalOptions {
            rule_names: ["no-let".into()].into_iter().collect(),
            allow_let_in_for_loop_init: true,
            ..FunctionalOptions::default()
        };
        assert!(
            scan_functional("for (let i = 0; i < 1; i++) {}", "fixture.ts", &options).is_empty()
        );

        options = FunctionalOptions {
            rule_names: ["no-try-statements".into()].into_iter().collect(),
            allow_try_catch: true,
            allow_try_finally: true,
            ..FunctionalOptions::default()
        };
        assert!(
            scan_functional(
                "try { work(); } catch (error) {} finally { cleanup(); }",
                "fixture.ts",
                &options,
            )
            .is_empty()
        );

        options = FunctionalOptions {
            rule_names: ["no-throw-statements".into()].into_iter().collect(),
            allow_throw_to_reject_promises: true,
            ..FunctionalOptions::default()
        };
        assert!(
            scan_functional(
                "async function f(error) { throw error; }",
                "fixture.ts",
                &options
            )
            .is_empty()
        );
    }
}
