#![doc = "Rust implementation of eslint-plugin-mocha rule logic."]

use std::fmt::{Arguments, Write as _};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionExpression, BindingPattern, CallExpression,
    ChainElement, Class, ClassElement, ConditionalExpression, Declaration,
    ExportDefaultDeclarationKind, Expression, Function, FunctionBody, PropertyKey, Statement,
    StaticMemberExpression,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};
use regex::Regex;

pub const RULE_NAMES: [&str; 24] = [
    "consistent-interface",
    "consistent-spacing-between-blocks",
    "handle-done-callback",
    "max-top-level-suites",
    "no-async-suite",
    "no-empty-title",
    "no-exclusive-tests",
    "no-exports",
    "no-global-tests",
    "no-hooks",
    "no-hooks-for-single-case",
    "no-identical-title",
    "no-mocha-arrows",
    "no-nested-tests",
    "no-pending-tests",
    "no-return-and-callback",
    "no-return-from-async",
    "no-setup-in-describe",
    "no-sibling-hooks",
    "no-synchronous-tests",
    "no-top-level-hooks",
    "prefer-arrow-callback",
    "valid-suite-title",
    "valid-test-title",
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
pub struct MochaOptions {
    pub consistent_interface: CompactString,
    pub max_top_level_suites_limit: u32,
    pub handle_done_ignore_pending: bool,
    pub no_hooks_allowed: SmallVec<[CompactString; 4]>,
    pub no_hooks_for_single_case_allowed: SmallVec<[CompactString; 4]>,
    pub no_synchronous_allowed: SmallVec<[CompactString; 3]>,
    pub no_empty_title_message: Option<CompactString>,
    pub valid_suite_title_pattern: Option<CompactString>,
    pub valid_suite_title_message: Option<CompactString>,
    pub valid_test_title_pattern: Option<CompactString>,
    pub valid_test_title_message: Option<CompactString>,
    pub prefer_arrow_allow_named_functions: bool,
    pub prefer_arrow_allow_unbound_this: bool,
}

impl Default for MochaOptions {
    fn default() -> Self {
        let mut no_synchronous_allowed = SmallVec::new();
        no_synchronous_allowed.push("async".into());
        no_synchronous_allowed.push("callback".into());
        no_synchronous_allowed.push("promise".into());
        Self {
            consistent_interface: "BDD".into(),
            max_top_level_suites_limit: 1,
            handle_done_ignore_pending: false,
            no_hooks_allowed: SmallVec::new(),
            no_hooks_for_single_case_allowed: SmallVec::new(),
            no_synchronous_allowed,
            no_empty_title_message: None,
            valid_suite_title_pattern: None,
            valid_suite_title_message: None,
            valid_test_title_pattern: None,
            valid_test_title_message: None,
            prefer_arrow_allow_named_functions: false,
            prefer_arrow_allow_unbound_this: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntityType {
    Suite,
    TestCase,
    Hook,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MochaInterface {
    Bdd,
    Tdd,
}

impl MochaInterface {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bdd => "BDD",
            Self::Tdd => "TDD",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Modifier {
    Pending,
    Exclusive,
}

#[derive(Clone, Copy)]
struct Callback<'a> {
    span: Span,
    body: CallbackBody<'a>,
    async_function: bool,
    arrow: bool,
    named_function: bool,
    params_len: usize,
    first_param_name: Option<&'a str>,
}

#[derive(Clone, Copy)]
enum CallbackBody<'a> {
    Function(&'a FunctionBody<'a>),
}

#[derive(Clone)]
struct Entity<'a> {
    name: CompactString,
    entity_type: EntityType,
    interface: MochaInterface,
    modifier: Option<Modifier>,
    span: Span,
    title: Option<CompactString>,
    callback: Option<Callback<'a>>,
}

#[derive(Default)]
struct Layer {
    suite_titles: FastHashMap<CompactString, Span>,
    test_titles: FastHashMap<CompactString, Span>,
    hook_names: FastHashMap<CompactString, Span>,
    hooks: SmallVec<[(CompactString, Span); 4]>,
    test_count: u32,
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

pub fn implemented_mocha_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_mocha(
    source_text: &str,
    filename: &str,
    options: &MochaOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let valid_suite_regex = options
        .valid_suite_title_pattern
        .as_ref()
        .and_then(|pattern| Regex::new(pattern.as_str()).ok());
    let valid_test_regex = options
        .valid_test_title_pattern
        .as_ref()
        .and_then(|pattern| Regex::new(pattern.as_str()).ok());
    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        options,
        valid_suite_regex,
        valid_test_regex,
        layers: SmallVec::new(),
        suite_depth: 0,
        test_depth: 0,
        top_level_suites: 0,
        has_test_entity: false,
        export_spans: SmallVec::new(),
    };
    scanner.layers.push(Layer::default());
    scanner.scan_statement_list(&parser_return.program.body, ContextKind::Program, true);
    scanner.finish_program();
    scanner.diagnostics
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContextKind {
    Program,
    SuiteCallback,
    TestCallback,
    HookCallback,
    Other,
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 16]>,
    options: &'a MochaOptions,
    valid_suite_regex: Option<Regex>,
    valid_test_regex: Option<Regex>,
    layers: SmallVec<[Layer; 8]>,
    suite_depth: u32,
    test_depth: u32,
    top_level_suites: u32,
    has_test_entity: bool,
    export_spans: SmallVec<[Span; 8]>,
}

impl<'a> Scanner<'a> {
    fn report(&mut self, rule_name: &'static str, message: impl Into<CompactString>, span: Span) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message: message.into(),
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }

    fn finish_program(&mut self) {
        if self.has_test_entity {
            let spans = self.export_spans.clone();
            for span in spans {
                self.report("no-exports", "Unexpected export from a test file", span);
            }
        }
    }

    fn scan_statement_list(
        &mut self,
        statements: &'a [Statement<'a>],
        context: ContextKind,
        direct_spacing_scope: bool,
    ) {
        let mut previous_end_line = None;
        for statement in statements {
            if direct_spacing_scope {
                if let Some(entity_span) = direct_statement_mocha_span(statement)
                    && let Some(end_line) = previous_end_line
                {
                    let (start_line, _) = self
                        .line_index
                        .position_for_offset(self.source_text, entity_span.start);
                    if start_line.saturating_sub(end_line) < 2 {
                        self.report(
                            "consistent-spacing-between-blocks",
                            "Expected line break before this statement.",
                            entity_span,
                        );
                    }
                }
                previous_end_line = Some(
                    self.line_index
                        .position_for_offset(self.source_text, statement.span().end)
                        .0,
                );
            }
            self.scan_statement(statement, context);
        }
    }

    fn scan_statement(&mut self, statement: &'a Statement<'a>, context: ContextKind) {
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

    fn scan_declaration(&mut self, declaration: &'a Declaration<'a>, context: ContextKind) {
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

    fn scan_expression(&mut self, expression: &'a Expression<'a>, context: ContextKind) {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => {
                self.scan_call_expression(call, context);
            }
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
                self.scan_expression(&member.object, context);
                self.scan_expression(&member.expression, context);
            }
            Expression::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function);
            }
            Expression::FunctionExpression(function) => {
                self.scan_function(function);
            }
            Expression::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                        if property.computed {
                            self.scan_property_key(&property.key, context);
                        }
                        self.scan_expression(&property.value, context);
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
                self.scan_conditional_expression(expression, context);
            }
            Expression::AssignmentExpression(expression) => {
                self.scan_expression(&expression.right, context);
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
            _ => {}
        }
    }

    fn scan_array_element(
        &mut self,
        element: &'a ArrayExpressionElement<'a>,
        context: ContextKind,
    ) {
        match element {
            ArrayExpressionElement::CallExpression(call) => {
                self.scan_call_expression(call, context)
            }
            ArrayExpressionElement::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            ArrayExpressionElement::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, context);
                self.scan_expression(&member.expression, context);
            }
            ArrayExpressionElement::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function);
            }
            ArrayExpressionElement::FunctionExpression(function) => self.scan_function(function),
            ArrayExpressionElement::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, context);
                }
            }
            ArrayExpressionElement::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                        self.scan_expression(&property.value, context);
                    }
                }
            }
            ArrayExpressionElement::ConditionalExpression(expression) => {
                self.scan_conditional_expression(expression, context);
            }
            _ => {}
        }
    }

    fn scan_argument(&mut self, argument: &'a Argument<'a>, context: ContextKind) {
        match argument {
            Argument::CallExpression(call) => self.scan_call_expression(call, context),
            Argument::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context)
            }
            Argument::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, context);
                self.scan_expression(&member.expression, context);
            }
            Argument::ArrowFunctionExpression(function) => self.scan_arrow_function(function),
            Argument::FunctionExpression(function) => self.scan_function(function),
            Argument::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, context);
                }
            }
            Argument::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                        self.scan_expression(&property.value, context);
                    }
                }
            }
            Argument::ConditionalExpression(expression) => {
                self.scan_conditional_expression(expression, context);
            }
            _ => {}
        }
    }

    fn scan_property_key(&mut self, key: &'a PropertyKey<'a>, context: ContextKind) {
        match key {
            PropertyKey::CallExpression(call) => self.scan_call_expression(call, context),
            PropertyKey::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context)
            }
            PropertyKey::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, context);
                self.scan_expression(&member.expression, context);
            }
            _ => {}
        }
    }

    fn scan_conditional_expression(
        &mut self,
        expression: &'a ConditionalExpression<'a>,
        context: ContextKind,
    ) {
        self.scan_expression(&expression.test, context);
        self.scan_expression(&expression.consequent, context);
        self.scan_expression(&expression.alternate, context);
    }

    fn scan_static_member_expression(
        &mut self,
        member: &'a StaticMemberExpression<'a>,
        context: ContextKind,
    ) {
        self.scan_expression(&member.object, context);
    }

    fn scan_call_expression(&mut self, call: &'a CallExpression<'a>, context: ContextKind) {
        if let Some(entity) = self.entity_for_call(call) {
            self.handle_entity(&entity);
            self.scan_entity_callback(&entity);
        } else {
            if context == ContextKind::SuiteCallback && !is_suite_config_call(call) {
                self.report(
                    "no-setup-in-describe",
                    "Unexpected function call in describe block.",
                    call.span,
                );
            }
            self.scan_expression(&call.callee, context);
            for argument in &call.arguments {
                self.scan_argument(argument, context);
            }
        }
    }

    fn scan_function(&mut self, function: &'a Function<'a>) {
        if let Some(body) = &function.body {
            self.scan_statement_list(&body.statements, ContextKind::Other, false);
        }
    }

    fn scan_arrow_function(&mut self, function: &'a ArrowFunctionExpression<'a>) {
        self.scan_statement_list(&function.body.statements, ContextKind::Other, false);
    }

    fn scan_class(&mut self, class: &'a Class<'a>) {
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    self.scan_statement_list(&block.body, ContextKind::Other, false);
                }
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value);
                }
                ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ContextKind::Other);
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_entity(&mut self, entity: &Entity<'a>) {
        self.has_test_entity = true;
        if entity.interface.as_str() != self.options.consistent_interface.as_str() {
            self.report(
                "consistent-interface",
                compact_format(format_args!(
                    "Unexpected use of {} interface instead of {}",
                    entity.interface.as_str(),
                    self.options.consistent_interface
                )),
                entity.span,
            );
        }

        if entity.entity_type == EntityType::Suite && self.suite_depth == 0 {
            self.top_level_suites += 1;
            if self.top_level_suites == self.options.max_top_level_suites_limit + 1 {
                self.report(
                    "max-top-level-suites",
                    compact_format(format_args!(
                        "The number of top-level suites is more than {}.",
                        self.options.max_top_level_suites_limit
                    )),
                    entity.span,
                );
            }
        }

        match entity.entity_type {
            EntityType::Suite => self.check_suite(entity),
            EntityType::TestCase => self.check_test_case(entity),
            EntityType::Hook => self.check_hook(entity),
        }

        if let Some(callback) = entity.callback {
            self.check_callback(entity, callback);
        } else if entity.entity_type == EntityType::TestCase
            && entity.modifier != Some(Modifier::Pending)
        {
            self.report(
                "no-pending-tests",
                "Unexpected pending mocha test.",
                entity.span,
            );
        }
    }

    fn check_suite(&mut self, entity: &Entity<'a>) {
        if entity.modifier == Some(Modifier::Exclusive) {
            self.report(
                "no-exclusive-tests",
                "Unexpected exclusive mocha test.",
                entity.span,
            );
        }
        if entity.modifier == Some(Modifier::Pending) {
            self.report(
                "no-pending-tests",
                "Unexpected pending mocha test.",
                entity.span,
            );
        }
        let invalid_suite_title = self.valid_suite_regex.as_ref().is_some_and(|pattern| {
            entity
                .title
                .as_ref()
                .is_some_and(|title| !pattern.is_match(title.as_str()))
        });
        self.check_title(
            "valid-suite-title",
            entity,
            invalid_suite_title,
            self.options.valid_suite_title_message.clone(),
            "Invalid \"",
        );
        self.check_empty_title(entity);
        if let Some(title) = &entity.title {
            let current = self.layers.last_mut().expect("root layer exists");
            if current.suite_titles.contains_key(title.as_str()) {
                self.report(
                    "no-identical-title",
                    compact_format(format_args!(
                        "Unexpected use of duplicate Mocha title `{title}`"
                    )),
                    entity.span,
                );
            } else {
                current.suite_titles.insert(title.clone(), entity.span);
            }
        }
    }

    fn check_test_case(&mut self, entity: &Entity<'a>) {
        if self.test_depth > 0 {
            self.report(
                "no-nested-tests",
                "Unexpected test nested inside another test.",
                entity.span,
            );
        }
        if self.suite_depth == 0 {
            self.report(
                "no-global-tests",
                "Unexpected global mocha test.",
                entity.span,
            );
        }
        if entity.modifier == Some(Modifier::Exclusive) {
            self.report(
                "no-exclusive-tests",
                "Unexpected exclusive mocha test.",
                entity.span,
            );
        }
        if entity.modifier == Some(Modifier::Pending) {
            self.report(
                "no-pending-tests",
                "Unexpected pending mocha test.",
                entity.span,
            );
        }
        let invalid_test_title = self.valid_test_regex.as_ref().is_some_and(|pattern| {
            entity
                .title
                .as_ref()
                .is_some_and(|title| !pattern.is_match(title.as_str()))
        });
        self.check_title(
            "valid-test-title",
            entity,
            invalid_test_title,
            self.options.valid_test_title_message.clone(),
            "Invalid \"",
        );
        self.check_empty_title(entity);
        let current = self.layers.last_mut().expect("root layer exists");
        current.test_count += 1;
        if let Some(title) = &entity.title {
            if current.test_titles.contains_key(title.as_str()) {
                self.report(
                    "no-identical-title",
                    compact_format(format_args!(
                        "Unexpected use of duplicate Mocha title `{title}`"
                    )),
                    entity.span,
                );
            } else {
                current.test_titles.insert(title.clone(), entity.span);
            }
        }
    }

    fn check_hook(&mut self, entity: &Entity<'a>) {
        if self.suite_depth == 0 {
            self.report(
                "no-top-level-hooks",
                compact_format(format_args!(
                    "Unexpected use of Mocha `{}` hook outside of a test suite",
                    display_call_name(entity.name.as_str())
                )),
                entity.span,
            );
        }
        if !self
            .options
            .no_hooks_allowed
            .iter()
            .any(|allowed| allowed.as_str() == entity.name.as_str())
        {
            self.report(
                "no-hooks",
                compact_format(format_args!(
                    "Unexpected use of Mocha `{}` hook",
                    display_call_name(entity.name.as_str())
                )),
                entity.span,
            );
        }
        let current = self.layers.last_mut().expect("root layer exists");
        current.hooks.push((entity.name.clone(), entity.span));
        if current.hook_names.contains_key(entity.name.as_str()) {
            self.report(
                "no-sibling-hooks",
                compact_format(format_args!(
                    "Unexpected use of duplicate Mocha `{}` hook",
                    display_call_name(entity.name.as_str())
                )),
                entity.span,
            );
        } else {
            current.hook_names.insert(entity.name.clone(), entity.span);
        }
    }

    fn check_callback(&mut self, entity: &Entity<'a>, callback: Callback<'a>) {
        if entity.entity_type == EntityType::Suite && callback.async_function {
            self.report(
                "no-async-suite",
                compact_format(format_args!(
                    "Unexpected async function in {}",
                    display_call_name(entity.name.as_str())
                )),
                callback.span,
            );
        }
        if callback.arrow {
            self.report(
                "no-mocha-arrows",
                "Unexpected arrow function.",
                callback.span,
            );
        } else if !self.callback_is_allowed_function_callback(callback) {
            self.report(
                "prefer-arrow-callback",
                "Unexpected function expression.",
                callback.span,
            );
        }
        if matches!(entity.entity_type, EntityType::TestCase | EntityType::Hook) {
            if let Some(param) = callback.first_param_name
                && !(entity.modifier == Some(Modifier::Pending)
                    && self.options.handle_done_ignore_pending)
                && !callback_body_calls_identifier(callback.body, param)
            {
                self.report(
                    "handle-done-callback",
                    compact_format(format_args!("Expected \"{param}\" callback to be handled.")),
                    callback.span,
                );
            }
            if callback.first_param_name.is_some() && callback_body_returns_value(callback.body) {
                self.report(
                    "no-return-and-callback",
                    "Unexpected use of `return` in a test with callback",
                    callback.span,
                );
            }
            if callback.async_function && callback_body_returns_value(callback.body) {
                self.report(
                    "no-return-from-async",
                    "Unexpected use of `return` in a test with an async function",
                    callback.span,
                );
            }
            if self.callback_is_synchronous(callback) {
                self.report(
                    "no-synchronous-tests",
                    "Unexpected synchronous test.",
                    callback.span,
                );
            }
        }
    }

    fn callback_is_synchronous(&self, callback: Callback<'a>) -> bool {
        let mut async_used = false;
        for method in &self.options.no_synchronous_allowed {
            match method.as_str() {
                "async" if callback.async_function => async_used = true,
                "callback" if callback.params_len == 1 => async_used = true,
                "promise" if callback_body_returns_promise(callback.body) => async_used = true,
                _ => {}
            }
        }
        !async_used
    }

    fn callback_is_allowed_function_callback(&self, callback: Callback<'a>) -> bool {
        (self.options.prefer_arrow_allow_named_functions && callback.named_function)
            || (self.options.prefer_arrow_allow_unbound_this
                && callback_body_contains_this(callback.body))
    }

    fn check_empty_title(&mut self, entity: &Entity<'a>) {
        let empty = entity
            .title
            .as_ref()
            .is_none_or(|title| title.trim().is_empty());
        if empty {
            self.report(
                "no-empty-title",
                self.options
                    .no_empty_title_message
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| "Unexpected empty test description.".into()),
                entity.span,
            );
        }
    }

    fn check_title(
        &mut self,
        rule_name: &'static str,
        entity: &Entity<'a>,
        invalid_title: bool,
        custom_message: Option<CompactString>,
        default_prefix: &str,
    ) {
        if !invalid_title {
            return;
        };
        self.report(
            rule_name,
            custom_message.unwrap_or_else(|| {
                compact_format(format_args!(
                    "{default_prefix}{}\" description found.",
                    display_call_name(entity.name.as_str())
                ))
            }),
            entity.span,
        );
    }

    fn scan_entity_callback(&mut self, entity: &Entity<'a>) {
        let Some(callback) = entity.callback else {
            return;
        };
        match entity.entity_type {
            EntityType::Suite => {
                self.suite_depth += 1;
                self.layers.push(Layer::default());
                self.scan_callback_body(callback.body, ContextKind::SuiteCallback);
                let layer = self.layers.pop().expect("suite layer exists");
                if layer.test_count == 1 {
                    for (hook_name, span) in layer.hooks {
                        if !self
                            .options
                            .no_hooks_for_single_case_allowed
                            .iter()
                            .any(|allowed| allowed.as_str() == hook_name.as_str())
                        {
                            self.report(
                                "no-hooks-for-single-case",
                                compact_format(format_args!(
                                    "Unexpected use of Mocha `{}` hook for a single test case",
                                    display_call_name(hook_name.as_str())
                                )),
                                span,
                            );
                        }
                    }
                }
                self.suite_depth = self.suite_depth.saturating_sub(1);
            }
            EntityType::TestCase => {
                self.test_depth += 1;
                self.scan_callback_body(callback.body, ContextKind::TestCallback);
                self.test_depth = self.test_depth.saturating_sub(1);
            }
            EntityType::Hook => {
                self.scan_callback_body(callback.body, ContextKind::HookCallback);
            }
        }
    }

    fn scan_callback_body(&mut self, body: CallbackBody<'a>, context: ContextKind) {
        match body {
            CallbackBody::Function(body) => self.scan_statement_list(
                &body.statements,
                context,
                context == ContextKind::SuiteCallback,
            ),
        }
    }

    fn entity_for_call(&self, call: &'a CallExpression<'a>) -> Option<Entity<'a>> {
        let path = call_path(call)?;
        let (name, entity_type, interface, modifier) = classify_mocha_path(&path)?;
        Some(Entity {
            name: CompactString::from(name),
            entity_type,
            interface,
            modifier,
            span: call.span,
            title: call
                .arguments
                .first()
                .and_then(argument_string_value)
                .map(CompactString::from),
            callback: call.arguments.last().and_then(callback_from_argument),
        })
    }
}

fn direct_statement_mocha_span(statement: &Statement<'_>) -> Option<Span> {
    match statement {
        Statement::ExpressionStatement(statement) => {
            direct_expression_mocha_span(&statement.expression)
        }
        _ => None,
    }
}

fn direct_expression_mocha_span(expression: &Expression<'_>) -> Option<Span> {
    match expression.get_inner_expression() {
        Expression::CallExpression(call) => {
            if call_path(call)
                .as_deref()
                .and_then(classify_mocha_path)
                .is_some()
            {
                return Some(call.span);
            }
            direct_expression_mocha_span(&call.callee)
                .or_else(|| call.arguments.iter().find_map(direct_argument_mocha_span))
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => {
                if call_path(call)
                    .as_deref()
                    .and_then(classify_mocha_path)
                    .is_some()
                {
                    Some(call.span)
                } else {
                    direct_expression_mocha_span(&call.callee)
                }
            }
            ChainElement::StaticMemberExpression(member) => {
                direct_expression_mocha_span(&member.object)
            }
            _ => None,
        },
        Expression::StaticMemberExpression(member) => direct_expression_mocha_span(&member.object),
        Expression::ComputedMemberExpression(member) => {
            direct_expression_mocha_span(&member.object)
        }
        _ => None,
    }
}

fn direct_argument_mocha_span(argument: &Argument<'_>) -> Option<Span> {
    match argument {
        Argument::CallExpression(call) => {
            if call_path(call)
                .as_deref()
                .and_then(classify_mocha_path)
                .is_some()
            {
                Some(call.span)
            } else {
                direct_expression_mocha_span(&call.callee)
            }
        }
        Argument::StaticMemberExpression(member) => direct_expression_mocha_span(&member.object),
        Argument::ComputedMemberExpression(member) => direct_expression_mocha_span(&member.object),
        _ => None,
    }
}

fn compact_format(args: Arguments<'_>) -> CompactString {
    let mut message = CompactString::new("");
    let _ = message.write_fmt(args);
    message
}

fn display_call_name(name: &str) -> CompactString {
    compact_format(format_args!("{name}()"))
}

fn classify_mocha_path(
    path: &[&str],
) -> Option<(&'static str, EntityType, MochaInterface, Option<Modifier>)> {
    let first = *path.first()?;
    let last = *path.last()?;
    let modifier = if last == "only" {
        Some(Modifier::Exclusive)
    } else if last == "skip" {
        Some(Modifier::Pending)
    } else {
        None
    };
    let base = if path.len() == 1 || (modifier.is_some() && path.len() == 2) {
        first
    } else {
        return None;
    };

    match base {
        "describe" => Some(("describe", EntityType::Suite, MochaInterface::Bdd, modifier)),
        "context" => Some(("context", EntityType::Suite, MochaInterface::Bdd, modifier)),
        "suite" => Some(("suite", EntityType::Suite, MochaInterface::Tdd, modifier)),
        "it" => Some(("it", EntityType::TestCase, MochaInterface::Bdd, modifier)),
        "specify" => Some((
            "specify",
            EntityType::TestCase,
            MochaInterface::Bdd,
            modifier,
        )),
        "test" => Some(("test", EntityType::TestCase, MochaInterface::Tdd, modifier)),
        "before" | "after" | "beforeEach" | "afterEach" => {
            let hook = match base {
                "before" => "before",
                "after" => "after",
                "beforeEach" => "beforeEach",
                _ => "afterEach",
            };
            Some((hook, EntityType::Hook, MochaInterface::Bdd, None))
        }
        "suiteSetup" | "suiteTeardown" | "setup" | "teardown" => {
            let hook = match base {
                "suiteSetup" => "suiteSetup",
                "suiteTeardown" => "suiteTeardown",
                "setup" => "setup",
                _ => "teardown",
            };
            Some((hook, EntityType::Hook, MochaInterface::Tdd, None))
        }
        "xdescribe" => Some((
            "xdescribe",
            EntityType::Suite,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        "xcontext" => Some((
            "xcontext",
            EntityType::Suite,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        "xit" => Some((
            "xit",
            EntityType::TestCase,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        "xspecify" => Some((
            "xspecify",
            EntityType::TestCase,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        _ => None,
    }
}

fn call_path<'a>(call: &'a CallExpression<'a>) -> Option<SmallVec<[&'a str; 3]>> {
    let mut path = SmallVec::new();
    collect_callee_path(call.callee.get_inner_expression(), &mut path)?;
    Some(path)
}

fn collect_callee_path<'a>(
    expression: &'a Expression<'a>,
    path: &mut SmallVec<[&'a str; 3]>,
) -> Option<()> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => {
            path.push(identifier.name.as_str());
            Some(())
        }
        Expression::StaticMemberExpression(member) => {
            collect_callee_path(&member.object, path)?;
            path.push(member.property.name.as_str());
            Some(())
        }
        _ => None,
    }
}

fn callback_from_argument<'a>(argument: &'a Argument<'a>) -> Option<Callback<'a>> {
    match argument {
        Argument::FunctionExpression(function) => {
            let body = function.body.as_deref()?;
            Some(Callback {
                span: function.span,
                body: CallbackBody::Function(body),
                async_function: function.r#async,
                arrow: false,
                named_function: function.id.is_some(),
                params_len: function.params.items.len(),
                first_param_name: function
                    .params
                    .items
                    .first()
                    .and_then(|param| binding_identifier_name(&param.pattern)),
            })
        }
        Argument::ArrowFunctionExpression(function) => Some(Callback {
            span: function.span,
            body: CallbackBody::Function(&function.body),
            async_function: function.r#async,
            arrow: true,
            named_function: false,
            params_len: function.params.items.len(),
            first_param_name: function
                .params
                .items
                .first()
                .and_then(|param| binding_identifier_name(&param.pattern)),
        }),
        _ => None,
    }
}

fn binding_identifier_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn argument_string_value<'a>(argument: &'a Argument<'a>) -> Option<&'a str> {
    match argument {
        Argument::StringLiteral(literal) => Some(literal.value.as_str()),
        Argument::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .map(|value| value.as_str()),
        _ => None,
    }
}

fn is_suite_config_call(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    matches!(
        member.object.get_inner_expression(),
        Expression::ThisExpression(_)
    ) && matches!(
        member.property.name.as_str(),
        "timeout" | "slow" | "retries"
    )
}

fn callback_body_calls_identifier(body: CallbackBody<'_>, name: &str) -> bool {
    match body {
        CallbackBody::Function(body) => body
            .statements
            .iter()
            .any(|statement| statement_calls_identifier(statement, name)),
    }
}

fn callback_body_returns_value(body: CallbackBody<'_>) -> bool {
    match body {
        CallbackBody::Function(body) => body.statements.iter().any(statement_returns_value),
    }
}

fn callback_body_returns_promise(body: CallbackBody<'_>) -> bool {
    match body {
        CallbackBody::Function(body) => body.statements.iter().any(|statement| {
            matches!(statement, Statement::ReturnStatement(statement) if statement.argument.as_ref().is_some_and(non_literal_expression))
        }),
    }
}

fn callback_body_contains_this(body: CallbackBody<'_>) -> bool {
    match body {
        CallbackBody::Function(body) => body.statements.iter().any(statement_contains_this),
    }
}

fn statement_returns_value(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ReturnStatement(statement) => statement.argument.is_some(),
        Statement::BlockStatement(block) => block.body.iter().any(statement_returns_value),
        Statement::IfStatement(statement) => {
            statement_returns_value(&statement.consequent)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| statement_returns_value(alternate))
        }
        _ => false,
    }
}

fn non_literal_expression(expression: &Expression<'_>) -> bool {
    !matches!(
        expression.get_inner_expression(),
        Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::RegExpLiteral(_)
    )
}

fn statement_contains_this(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => {
            expression_contains_this(&statement.expression)
        }
        Statement::BlockStatement(block) => block.body.iter().any(statement_contains_this),
        Statement::ReturnStatement(statement) => statement
            .argument
            .as_ref()
            .is_some_and(expression_contains_this),
        Statement::IfStatement(statement) => {
            expression_contains_this(&statement.test)
                || statement_contains_this(&statement.consequent)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| statement_contains_this(alternate))
        }
        Statement::ThrowStatement(statement) => expression_contains_this(&statement.argument),
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                declarator
                    .init
                    .as_ref()
                    .is_some_and(expression_contains_this)
            })
        }
        Statement::WhileStatement(statement) => {
            expression_contains_this(&statement.test) || statement_contains_this(&statement.body)
        }
        Statement::DoWhileStatement(statement) => {
            statement_contains_this(&statement.body) || expression_contains_this(&statement.test)
        }
        Statement::ForStatement(statement) => {
            statement
                .test
                .as_ref()
                .is_some_and(expression_contains_this)
                || statement
                    .update
                    .as_ref()
                    .is_some_and(expression_contains_this)
                || statement_contains_this(&statement.body)
        }
        Statement::ForInStatement(statement) => {
            expression_contains_this(&statement.right) || statement_contains_this(&statement.body)
        }
        Statement::ForOfStatement(statement) => {
            expression_contains_this(&statement.right) || statement_contains_this(&statement.body)
        }
        Statement::SwitchStatement(statement) => {
            expression_contains_this(&statement.discriminant)
                || statement.cases.iter().any(|case| {
                    case.test.as_ref().is_some_and(expression_contains_this)
                        || case.consequent.iter().any(statement_contains_this)
                })
        }
        Statement::TryStatement(statement) => {
            statement.block.body.iter().any(statement_contains_this)
                || statement
                    .handler
                    .as_ref()
                    .is_some_and(|handler| handler.body.body.iter().any(statement_contains_this))
                || statement
                    .finalizer
                    .as_ref()
                    .is_some_and(|finalizer| finalizer.body.iter().any(statement_contains_this))
        }
        _ => false,
    }
}

fn expression_contains_this(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::ThisExpression(_) => true,
        Expression::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => {
                expression_contains_this(&call.callee)
                    || call.arguments.iter().any(argument_contains_this)
            }
            ChainElement::StaticMemberExpression(member) => {
                expression_contains_this(&member.object)
            }
            ChainElement::ComputedMemberExpression(member) => {
                expression_contains_this(&member.object)
                    || expression_contains_this(&member.expression)
            }
            _ => false,
        },
        Expression::StaticMemberExpression(member) => expression_contains_this(&member.object),
        Expression::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        Expression::UnaryExpression(expression) => expression_contains_this(&expression.argument),
        Expression::AwaitExpression(expression) => expression_contains_this(&expression.argument),
        Expression::BinaryExpression(expression) => {
            expression_contains_this(&expression.left)
                || expression_contains_this(&expression.right)
        }
        Expression::LogicalExpression(expression) => {
            expression_contains_this(&expression.left)
                || expression_contains_this(&expression.right)
        }
        Expression::ConditionalExpression(expression) => {
            expression_contains_this(&expression.test)
                || expression_contains_this(&expression.consequent)
                || expression_contains_this(&expression.alternate)
        }
        Expression::AssignmentExpression(expression) => expression_contains_this(&expression.right),
        Expression::SequenceExpression(expression) => {
            expression.expressions.iter().any(expression_contains_this)
        }
        Expression::TemplateLiteral(template) => {
            template.expressions.iter().any(expression_contains_this)
        }
        Expression::TaggedTemplateExpression(expression) => {
            expression_contains_this(&expression.tag)
                || expression
                    .quasi
                    .expressions
                    .iter()
                    .any(expression_contains_this)
        }
        Expression::ArrayExpression(expression) => {
            expression.elements.iter().any(array_element_contains_this)
        }
        Expression::ObjectExpression(expression) => expression.properties.iter().any(|property| {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                (property.computed && property_key_contains_this(&property.key))
                    || expression_contains_this(&property.value)
            } else {
                false
            }
        }),
        _ => false,
    }
}

fn argument_contains_this(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        Argument::StaticMemberExpression(member) => expression_contains_this(&member.object),
        Argument::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        Argument::ArrayExpression(expression) => {
            expression.elements.iter().any(array_element_contains_this)
        }
        Argument::ObjectExpression(expression) => expression.properties.iter().any(|property| {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                expression_contains_this(&property.value)
            } else {
                false
            }
        }),
        Argument::ConditionalExpression(expression) => {
            expression_contains_this(&expression.test)
                || expression_contains_this(&expression.consequent)
                || expression_contains_this(&expression.alternate)
        }
        _ => false,
    }
}

fn array_element_contains_this(element: &ArrayExpressionElement<'_>) -> bool {
    match element {
        ArrayExpressionElement::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        ArrayExpressionElement::StaticMemberExpression(member) => {
            expression_contains_this(&member.object)
        }
        ArrayExpressionElement::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        ArrayExpressionElement::ArrayExpression(expression) => {
            expression.elements.iter().any(array_element_contains_this)
        }
        ArrayExpressionElement::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                    expression_contains_this(&property.value)
                } else {
                    false
                }
            })
        }
        ArrayExpressionElement::ConditionalExpression(expression) => {
            expression_contains_this(&expression.test)
                || expression_contains_this(&expression.consequent)
                || expression_contains_this(&expression.alternate)
        }
        _ => false,
    }
}

fn property_key_contains_this(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        PropertyKey::StaticMemberExpression(member) => expression_contains_this(&member.object),
        PropertyKey::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        _ => false,
    }
}

fn statement_calls_identifier(statement: &Statement<'_>, name: &str) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => {
            expression_calls_identifier(&statement.expression, name)
        }
        Statement::BlockStatement(block) => block
            .body
            .iter()
            .any(|statement| statement_calls_identifier(statement, name)),
        Statement::ReturnStatement(statement) => statement
            .argument
            .as_ref()
            .is_some_and(|argument| expression_calls_identifier(argument, name)),
        Statement::IfStatement(statement) => {
            expression_calls_identifier(&statement.test, name)
                || statement_calls_identifier(&statement.consequent, name)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| statement_calls_identifier(alternate, name))
        }
        _ => false,
    }
}

fn expression_calls_identifier(expression: &Expression<'_>, name: &str) -> bool {
    match expression.get_inner_expression() {
        Expression::CallExpression(call) => {
            matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name.as_str() == name)
                || call
                    .arguments
                    .iter()
                    .any(|argument| argument_calls_identifier(argument, name))
        }
        Expression::StaticMemberExpression(member) => {
            expression_calls_identifier(&member.object, name)
        }
        Expression::ComputedMemberExpression(member) => {
            expression_calls_identifier(&member.object, name)
                || expression_calls_identifier(&member.expression, name)
        }
        Expression::BinaryExpression(expression) => {
            expression_calls_identifier(&expression.left, name)
                || expression_calls_identifier(&expression.right, name)
        }
        Expression::LogicalExpression(expression) => {
            expression_calls_identifier(&expression.left, name)
                || expression_calls_identifier(&expression.right, name)
        }
        Expression::ConditionalExpression(expression) => {
            expression_calls_identifier(&expression.test, name)
                || expression_calls_identifier(&expression.consequent, name)
                || expression_calls_identifier(&expression.alternate, name)
        }
        _ => false,
    }
}

fn argument_calls_identifier(argument: &Argument<'_>, name: &str) -> bool {
    match argument {
        Argument::CallExpression(call) => {
            matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name.as_str() == name)
        }
        Argument::Identifier(identifier) => identifier.name.as_str() == name,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{MochaOptions, scan_mocha};

    fn rule_names(source_text: &str) -> oxlint_plugins_carton::SmallVec<[&'static str; 16]> {
        scan_mocha(source_text, "fixture.test.js", &MochaOptions::default())
            .into_iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect()
    }

    #[test]
    fn scans_core_mocha_rules() {
        let rules = rule_names(
            r#"
            beforeEach(function () {});
            it("global", function () {});
            describe.only("", async function () {
              before(function (done) {});
              before(function () {});
              it("works", function (done) { return fetch("/"); });
              it("works", () => {});
              it.skip("later");
              it("async return", async function () { return fetch("/"); });
              it("nested", function () { it("bad", function () {}); });
              helper();
            });
            describe("single", function () {
              before(function () {});
              it("one", function (done) { done(); });
            });
            suite("tdd", function () { test("bad", function () {}); });
            export const value = 1;
            "#,
        );

        assert!(rules.contains(&"consistent-spacing-between-blocks"));
        assert!(rules.contains(&"no-top-level-hooks"));
        assert!(rules.contains(&"no-hooks"));
        assert!(rules.contains(&"no-exclusive-tests"));
        assert!(rules.contains(&"no-empty-title"));
        assert!(rules.contains(&"no-async-suite"));
        assert!(rules.contains(&"handle-done-callback"));
        assert!(rules.contains(&"no-sibling-hooks"));
        assert!(rules.contains(&"no-return-and-callback"));
        assert!(rules.contains(&"no-return-from-async"));
        assert!(rules.contains(&"no-mocha-arrows"));
        assert!(rules.contains(&"no-pending-tests"));
        assert!(rules.contains(&"no-global-tests"));
        assert!(rules.contains(&"no-hooks-for-single-case"));
        assert!(rules.contains(&"no-nested-tests"));
        assert!(rules.contains(&"no-setup-in-describe"));
        assert!(rules.contains(&"no-synchronous-tests"));
        assert!(rules.contains(&"consistent-interface"));
        assert!(rules.contains(&"no-exports"));
    }

    #[test]
    fn scans_title_rules_with_options() {
        let options = MochaOptions {
            valid_suite_title_pattern: Some("^Suite".into()),
            valid_test_title_pattern: Some("^should".into()),
            ..MochaOptions::default()
        };
        let rules = scan_mocha(
            r#"
            describe("bad suite", function () {
              it("bad test", function () {});
            });
            "#,
            "fixture.test.js",
            &options,
        )
        .into_iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect::<oxlint_plugins_carton::SmallVec<[&'static str; 8]>>();

        assert!(rules.contains(&"valid-suite-title"));
        assert!(rules.contains(&"valid-test-title"));
    }
}
