//! AST scanner for the mocha port. Contains the `Scanner` struct and
//! every traversal / rule check method as an `impl Scanner` block.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use std::fmt::Write as _;

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};
use regex::Regex;

use crate::helpers::*;
use crate::{
    Callback, CallbackBody, ContextKind, Diagnostic, Entity, EntityType, Layer, LineIndex,
    MochaInterface, MochaOptions, Modifier,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    pub(crate) options: &'a MochaOptions,
    pub(crate) valid_suite_regex: Option<Regex>,
    pub(crate) valid_test_regex: Option<Regex>,
    pub(crate) layers: SmallVec<[Layer; 8]>,
    pub(crate) suite_depth: u32,
    pub(crate) test_depth: u32,
    pub(crate) top_level_suites: u32,
    pub(crate) has_test_entity: bool,
    pub(crate) export_spans: SmallVec<[Span; 8]>,
}

impl<'a> Scanner<'a> {
    fn report(&mut self, rule_name: &'static str, message: impl Into<CompactString>, span: Span) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message: message.into(),
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }

    pub(crate) fn finish_program(&mut self) {
        if self.has_test_entity {
            let spans = self.export_spans.clone();
            for span in spans {
                self.report("no-exports", "Unexpected export from a test file", span);
            }
        }
    }

    pub(crate) fn scan_statement_list(
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
