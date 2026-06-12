#![doc = "Rust implementation of eslint-plugin-cypress rule logic."]

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrayPattern, ArrowFunctionExpression, AwaitExpression,
    BindingPattern, BindingRestElement, CallExpression, ChainElement, Class, ClassElement,
    ConditionalExpression, Declaration, ExportDefaultDeclarationKind, Expression, ForStatementInit,
    ForStatementLeft, Function, FunctionBody, ImportDeclaration, ImportDeclarationSpecifier,
    ObjectPropertyKind, PropertyKey, Statement, StaticMemberExpression, VariableDeclaration,
};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};

pub const RULE_NAMES: [&str; 13] = [
    "assertion-before-screenshot",
    "no-and",
    "no-assigning-return-values",
    "no-async-before",
    "no-async-tests",
    "no-chained-get",
    "no-debug",
    "no-force",
    "no-pause",
    "no-unnecessary-waiting",
    "no-xpath",
    "require-data-selectors",
    "unsafe-to-chain-command",
];

const ASSERTION_COMMANDS: [&str; 6] = [
    "should",
    "and",
    "contains",
    "get",
    "scrollIntoView",
    "scrollTo",
];
const ALLOW_AND_AFTER: [&str; 3] = ["should", "and", "contains"];
const ASSIGNMENT_ALLOWED_COMMANDS: [&str; 4] = ["now", "spy", "state", "stub"];
const FORCE_ACTION_COMMANDS: [&str; 8] = [
    "click",
    "dblclick",
    "type",
    "trigger",
    "check",
    "rightclick",
    "focus",
    "select",
];
const UNSAFE_CHAIN_ACTIONS: [&str; 19] = [
    "blur",
    "clear",
    "click",
    "check",
    "dblclick",
    "each",
    "focus",
    "rightclick",
    "screenshot",
    "scrollIntoView",
    "scrollTo",
    "select",
    "selectFile",
    "spread",
    "submit",
    "type",
    "trigger",
    "uncheck",
    "within",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CypressOptions {
    pub unsafe_to_chain_methods: SmallVec<[CompactString; 8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValueKind {
    Number,
    Other,
}

#[derive(Default)]
struct Scope {
    values: FastHashMap<CompactString, ValueKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParentKind {
    None,
    MemberObject,
    Other,
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

pub fn implemented_cypress_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_cypress(
    source_text: &str,
    filename: &str,
    options: &CypressOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        scopes: SmallVec::new(),
        data_selector_variables: FastHashMap::default(),
        unsafe_to_chain_methods: options.unsafe_to_chain_methods.clone(),
    };
    scanner.push_scope();
    scanner.scan_statement_list(&parser_return.program.body, None, false);
    scanner.diagnostics
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 16]>,
    scopes: SmallVec<[Scope; 8]>,
    data_selector_variables: FastHashMap<CompactString, bool>,
    unsafe_to_chain_methods: SmallVec<[CompactString; 8]>,
}

impl<'a> Scanner<'a> {
    fn push_scope(&mut self) {
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

    fn scan_statement_list(
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

fn call_static_member_name<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    match call.callee.get_inner_expression() {
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

fn call_static_member_object<'a>(call: &'a CallExpression<'a>) -> Option<&'a Expression<'a>> {
    match call.callee.get_inner_expression() {
        Expression::StaticMemberExpression(member) => Some(&member.object),
        _ => None,
    }
}

fn previous_command_in_chain<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    let Some(Expression::CallExpression(object_call)) = call_static_member_object(call) else {
        return None;
    };
    call_static_member_name(object_call)
}

fn expression_identifier_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn callee_identifier_name<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn cypress_command_names<'a>(call: &'a CallExpression<'a>) -> Option<(&'a str, &'a str)> {
    let mut names = SmallVec::<[&'a str; 8]>::new();
    collect_cypress_command_names(call, &mut names)?;
    let first = *names.first()?;
    let last = *names.last()?;
    Some((first, last))
}

fn collect_cypress_command_names<'a>(
    call: &'a CallExpression<'a>,
    names: &mut SmallVec<[&'a str; 8]>,
) -> Option<()> {
    let command = call_static_member_name(call)?;
    let object = call_static_member_object(call)?;

    if expression_identifier_name(object) == Some("cy") {
        names.push(command);
        return Some(());
    }

    if let Expression::CallExpression(object_call) = object.get_inner_expression() {
        collect_cypress_command_names(object_call, names)?;
        names.push(command);
        return Some(());
    }

    None
}

fn call_has_force_option<'a>(call: &'a CallExpression<'a>) -> bool {
    call.arguments.iter().any(|argument| {
        let Argument::ObjectExpression(object) = argument else {
            return false;
        };
        object.properties.iter().any(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            property_key_name(&property.key) == Some("force")
        })
    })
}

fn property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        PropertyKey::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn is_alias_or_data_selector(selector: &str) -> bool {
    selector.starts_with("[data-") || selector.starts_with('@')
}

fn argument_is_async_function(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::ArrowFunctionExpression(function) => function.r#async,
        Argument::FunctionExpression(function) => function.r#async,
        _ => false,
    }
}

fn argument_contains_cypress_identifier(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        Argument::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        _ => false,
    }
}

fn function_body_contains_cypress(body: &FunctionBody<'_>) -> bool {
    body.statements.iter().any(statement_contains_cypress)
}

fn statement_contains_cypress(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => {
            expression_contains_cypress(&statement.expression)
        }
        Statement::BlockStatement(block) => block.body.iter().any(statement_contains_cypress),
        Statement::IfStatement(statement) => {
            expression_contains_cypress(&statement.test)
                || statement_contains_cypress(&statement.consequent)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(statement_contains_cypress)
        }
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                declarator
                    .init
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
            })
        }
        Statement::ReturnStatement(statement) => statement
            .argument
            .as_ref()
            .is_some_and(expression_contains_cypress),
        Statement::ThrowStatement(statement) => expression_contains_cypress(&statement.argument),
        Statement::WhileStatement(statement) => {
            expression_contains_cypress(&statement.test)
                || statement_contains_cypress(&statement.body)
        }
        Statement::DoWhileStatement(statement) => {
            statement_contains_cypress(&statement.body)
                || expression_contains_cypress(&statement.test)
        }
        Statement::ForStatement(statement) => {
            statement
                .test
                .as_ref()
                .is_some_and(expression_contains_cypress)
                || statement
                    .update
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
                || statement_contains_cypress(&statement.body)
        }
        Statement::ForInStatement(statement) => {
            expression_contains_cypress(&statement.right)
                || statement_contains_cypress(&statement.body)
        }
        Statement::ForOfStatement(statement) => {
            expression_contains_cypress(&statement.right)
                || statement_contains_cypress(&statement.body)
        }
        Statement::SwitchStatement(statement) => {
            expression_contains_cypress(&statement.discriminant)
                || statement.cases.iter().any(|case| {
                    case.test.as_ref().is_some_and(expression_contains_cypress)
                        || case.consequent.iter().any(statement_contains_cypress)
                })
        }
        Statement::TryStatement(statement) => {
            statement.block.body.iter().any(statement_contains_cypress)
                || statement
                    .handler
                    .as_ref()
                    .is_some_and(|handler| handler.body.body.iter().any(statement_contains_cypress))
                || statement
                    .finalizer
                    .as_ref()
                    .is_some_and(|finalizer| finalizer.body.iter().any(statement_contains_cypress))
        }
        Statement::FunctionDeclaration(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Statement::ClassDeclaration(class) => class_contains_cypress(class),
        Statement::ExportNamedDeclaration(declaration) => declaration
            .declaration
            .as_ref()
            .is_some_and(declaration_contains_cypress),
        Statement::ExportDefaultDeclaration(declaration) => match &declaration.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => function
                .body
                .as_deref()
                .is_some_and(function_body_contains_cypress),
            ExportDefaultDeclarationKind::ClassDeclaration(class) => class_contains_cypress(class),
            declaration => declaration
                .as_expression()
                .is_some_and(expression_contains_cypress),
        },
        _ => false,
    }
}

fn declaration_contains_cypress(declaration: &Declaration<'_>) -> bool {
    match declaration {
        Declaration::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                declarator
                    .init
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
            })
        }
        Declaration::FunctionDeclaration(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Declaration::ClassDeclaration(class) => class_contains_cypress(class),
        _ => false,
    }
}

fn class_contains_cypress(class: &Class<'_>) -> bool {
    class.body.body.iter().any(|element| match element {
        ClassElement::StaticBlock(block) => block.body.iter().any(statement_contains_cypress),
        ClassElement::MethodDefinition(method) => method
            .value
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        ClassElement::PropertyDefinition(property) => property
            .value
            .as_ref()
            .is_some_and(expression_contains_cypress),
        ClassElement::AccessorProperty(property) => property
            .value
            .as_ref()
            .is_some_and(expression_contains_cypress),
        ClassElement::TSIndexSignature(_) => false,
    })
}

fn expression_contains_cypress(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "cy" | "Cypress")
        }
        Expression::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        Expression::StaticMemberExpression(member) => expression_contains_cypress(&member.object),
        Expression::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        Expression::ChainExpression(chain) => chain_element_contains_cypress(&chain.expression),
        Expression::ParenthesizedExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSAsExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSSatisfiesExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSTypeAssertion(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSNonNullExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSInstantiationExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::AwaitExpression(expression) => {
            expression_contains_cypress(&expression.argument)
        }
        Expression::ArrayExpression(expression) => expression
            .elements
            .iter()
            .any(array_element_contains_cypress),
        Expression::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    expression_contains_cypress(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    expression_contains_cypress(&spread.argument)
                }
            })
        }
        Expression::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        Expression::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Expression::ClassExpression(class) => class_contains_cypress(class),
        Expression::AssignmentExpression(expression) => {
            expression_contains_cypress(&expression.right)
        }
        Expression::ConditionalExpression(expression) => {
            expression_contains_cypress(&expression.test)
                || expression_contains_cypress(&expression.consequent)
                || expression_contains_cypress(&expression.alternate)
        }
        Expression::BinaryExpression(expression) => {
            expression_contains_cypress(&expression.left)
                || expression_contains_cypress(&expression.right)
        }
        Expression::LogicalExpression(expression) => {
            expression_contains_cypress(&expression.left)
                || expression_contains_cypress(&expression.right)
        }
        Expression::SequenceExpression(expression) => expression
            .expressions
            .iter()
            .any(expression_contains_cypress),
        Expression::UnaryExpression(expression) => {
            expression_contains_cypress(&expression.argument)
        }
        Expression::YieldExpression(expression) => expression
            .argument
            .as_ref()
            .is_some_and(expression_contains_cypress),
        Expression::TaggedTemplateExpression(expression) => {
            expression_contains_cypress(&expression.tag)
                || expression
                    .quasi
                    .expressions
                    .iter()
                    .any(expression_contains_cypress)
        }
        Expression::TemplateLiteral(template) => {
            template.expressions.iter().any(expression_contains_cypress)
        }
        Expression::ImportExpression(expression) => {
            expression_contains_cypress(&expression.source)
                || expression
                    .options
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
        }
        _ => false,
    }
}

fn chain_element_contains_cypress(element: &ChainElement<'_>) -> bool {
    match element {
        ChainElement::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        ChainElement::StaticMemberExpression(member) => expression_contains_cypress(&member.object),
        ChainElement::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        ChainElement::TSNonNullExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        ChainElement::PrivateFieldExpression(member) => expression_contains_cypress(&member.object),
    }
}

fn argument_expression_contains_cypress(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::SpreadElement(spread) => expression_contains_cypress(&spread.argument),
        Argument::Identifier(identifier) => matches!(identifier.name.as_str(), "cy" | "Cypress"),
        Argument::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        Argument::StaticMemberExpression(member) => expression_contains_cypress(&member.object),
        Argument::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        Argument::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        Argument::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Argument::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    expression_contains_cypress(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    expression_contains_cypress(&spread.argument)
                }
            })
        }
        Argument::ArrayExpression(expression) => expression
            .elements
            .iter()
            .any(array_element_contains_cypress),
        Argument::ConditionalExpression(expression) => {
            expression_contains_cypress(&expression.test)
                || expression_contains_cypress(&expression.consequent)
                || expression_contains_cypress(&expression.alternate)
        }
        Argument::AwaitExpression(expression) => expression_contains_cypress(&expression.argument),
        Argument::SequenceExpression(expression) => expression
            .expressions
            .iter()
            .any(expression_contains_cypress),
        Argument::TaggedTemplateExpression(expression) => {
            expression_contains_cypress(&expression.tag)
                || expression
                    .quasi
                    .expressions
                    .iter()
                    .any(expression_contains_cypress)
        }
        Argument::TemplateLiteral(template) => {
            template.expressions.iter().any(expression_contains_cypress)
        }
        _ => false,
    }
}

fn array_element_contains_cypress(element: &ArrayExpressionElement<'_>) -> bool {
    match element {
        ArrayExpressionElement::SpreadElement(spread) => {
            expression_contains_cypress(&spread.argument)
        }
        ArrayExpressionElement::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "cy" | "Cypress")
        }
        ArrayExpressionElement::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        ArrayExpressionElement::StaticMemberExpression(member) => {
            expression_contains_cypress(&member.object)
        }
        ArrayExpressionElement::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        ArrayExpressionElement::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        ArrayExpressionElement::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        ArrayExpressionElement::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    expression_contains_cypress(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    expression_contains_cypress(&spread.argument)
                }
            })
        }
        ArrayExpressionElement::ArrayExpression(expression) => expression
            .elements
            .iter()
            .any(array_element_contains_cypress),
        ArrayExpressionElement::ConditionalExpression(expression) => {
            expression_contains_cypress(&expression.test)
                || expression_contains_cypress(&expression.consequent)
                || expression_contains_cypress(&expression.alternate)
        }
        ArrayExpressionElement::Elision(_) => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{CypressOptions, scan_cypress};
    use oxlint_plugins_carton::SmallVec;

    fn rule_names(source_text: &str) -> SmallVec<[&'static str; 16]> {
        scan_cypress(source_text, "fixture.tsx", &CypressOptions::default())
            .into_iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect()
    }

    #[test]
    fn scans_core_cypress_rules() {
        let rules = rule_names(
            r#"
            const a = cy.get(".foo");
            before("x", async () => { cy.get(".foo"); });
            it("x", async () => { Cypress.env("x"); });
            cy.get(".foo").and("be.visible");
            cy.debug();
            cy.pause();
            cy.xpath("//main");
            cy.wait(100);
            cy.get(".foo").click({ force: true }).should("exist");
            cy.visit("/home");
            cy.screenshot();
            "#,
        );

        assert!(rules.contains(&"no-assigning-return-values"));
        assert!(rules.contains(&"no-async-before"));
        assert!(rules.contains(&"no-async-tests"));
        assert!(rules.contains(&"no-and"));
        assert!(rules.contains(&"no-debug"));
        assert!(rules.contains(&"no-pause"));
        assert!(rules.contains(&"no-xpath"));
        assert!(rules.contains(&"no-unnecessary-waiting"));
        assert!(rules.contains(&"no-force"));
        assert!(rules.contains(&"unsafe-to-chain-command"));
        assert!(rules.contains(&"assertion-before-screenshot"));
        assert!(rules.contains(&"require-data-selectors"));
    }

    #[test]
    fn tracks_data_selector_variables_and_wait_defaults() {
        let rules = rule_names(
            r#"
            const GOOD = "[data-cy=submit]";
            cy.get(GOOD);
            function customWait({ ms = 1 }) { cy.wait(ms); }
            "#,
        );

        assert!(!rules.contains(&"require-data-selectors"));
        assert!(rules.contains(&"no-unnecessary-waiting"));
    }

    #[test]
    fn supports_custom_unsafe_methods() {
        let mut options = CypressOptions::default();
        options.unsafe_to_chain_methods.push("customType".into());
        let rules = scan_cypress(
            r#"cy.get("new-todo").customType("todo").should("have.class", "active");"#,
            "fixture.ts",
            &options,
        )
        .into_iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect::<SmallVec<[_; 16]>>();

        assert!(rules.contains(&"unsafe-to-chain-command"));
    }
}
