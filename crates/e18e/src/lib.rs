#![doc = "Rust implementation of @e18e/eslint-plugin rule logic."]
#![allow(
    clippy::collapsible_if,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::needless_borrow,
    clippy::question_mark,
    reason = "The e18e port builds many small autofix strings from source slices; keeping that string assembly local is clearer than adding broad formatting abstractions in the first native port."
)]

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, AssignmentTarget, CallExpression,
    ChainElement, Class, ClassElement, Declaration, Expression, ForStatementInit, ForStatementLeft,
    Function, FunctionBody, ImportDeclaration, NewExpression, ObjectPropertyKind, Program,
    PropertyKey, RegExpFlags, Statement,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub const RULE_NAMES: [&str; 25] = [
    "prefer-array-at",
    "prefer-array-fill",
    "prefer-array-from-map",
    "prefer-includes",
    "prefer-array-to-reversed",
    "prefer-array-to-sorted",
    "prefer-array-to-spliced",
    "prefer-exponentiation-operator",
    "prefer-nullish-coalescing",
    "prefer-object-has-own",
    "prefer-spread-syntax",
    "prefer-url-canparse",
    "no-indexof-equality",
    "prefer-timer-args",
    "prefer-date-now",
    "prefer-regex-test",
    "prefer-array-some",
    "prefer-static-regex",
    "prefer-inline-equality",
    "prefer-string-fromcharcode",
    "prefer-includes-over-regex-test",
    "no-delete-property",
    "no-spread-in-reduce",
    "prefer-static-collator",
    "ban-dependencies",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub array: Option<CompactString>,
    pub index: Option<CompactString>,
    pub item: Option<CompactString>,
    pub length: Option<CompactString>,
    pub value: Option<CompactString>,
    pub iterable: Option<CompactString>,
    pub mapper: Option<CompactString>,
    pub regex: Option<CompactString>,
    pub string: Option<CompactString>,
    pub original: Option<CompactString>,
    pub name: Option<CompactString>,
    pub replacement: Option<CompactString>,
    pub url: Option<CompactString>,
    pub description: Option<CompactString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BanDependency {
    pub module_name: CompactString,
    pub message_id: CompactString,
    pub replacement: Option<CompactString>,
    pub url: Option<CompactString>,
    pub description: Option<CompactString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct E18eOptions {
    pub rule_names: SmallVec<[CompactString; 25]>,
    pub banned_dependencies: SmallVec<[BanDependency; 16]>,
}

impl Default for E18eOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            banned_dependencies: SmallVec::new(),
        }
    }
}

impl E18eOptions {
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

pub fn implemented_e18e_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_e18e(
    source_text: &str,
    filename: &str,
    options: &E18eOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let line_index = LineIndex::new(source_text);
    if filename.ends_with("package.json") {
        return scan_package_json_dependencies(source_text, options, &line_index);
    }

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
        line_index,
        options,
        diagnostics: SmallVec::new(),
        function_depth: 0,
    };
    scanner.scan_program(&parser_return.program);
    scanner.diagnostics
}

fn scan_package_json_dependencies(
    source_text: &str,
    options: &E18eOptions,
    line_index: &LineIndex,
) -> SmallVec<[Diagnostic; 32]> {
    let mut diagnostics = SmallVec::new();
    if !options.has_rule("ban-dependencies") {
        return diagnostics;
    }

    for dependency in &options.banned_dependencies {
        let needle = format!("\"{}\"", dependency.module_name);
        let mut search_start = 0usize;
        while let Some(offset) = source_text[search_start..].find(&needle) {
            let start = search_start + offset;
            let span = Span::new(start as u32, (start + needle.len()) as u32);
            diagnostics.push(ban_dependency_diagnostic(
                dependency,
                span,
                source_text,
                line_index,
            ));
            search_start = start + needle.len();
        }
    }
    diagnostics
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    options: &'a E18eOptions,
    diagnostics: SmallVec<[Diagnostic; 32]>,
    function_depth: usize,
}

impl<'a> Scanner<'a> {
    fn scan_program(&mut self, program: &'a Program<'a>) {
        for statement in &program.body {
            self.scan_statement(statement);
        }
    }

    fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span, None);
    }

    fn report_with_fix(
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

    fn report_with_data(
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

    fn scan_statement(&mut self, statement: &'a Statement<'a>) {
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

    fn scan_variable_declaration(
        &mut self,
        declaration: &'a oxc_ast::ast::VariableDeclaration<'a>,
    ) {
        for declarator in &declaration.declarations {
            if let Some(init) = &declarator.init {
                self.scan_expression(init, ExprContext::Other);
            }
        }
    }

    fn scan_for_left(&mut self, left: &'a ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration);
        }
    }

    fn scan_function(&mut self, function: &'a Function<'a>) {
        self.function_depth += 1;
        if let Some(body) = &function.body {
            self.scan_function_body(body);
        }
        self.function_depth -= 1;
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>) {
        for statement in &body.statements {
            self.scan_statement(statement);
        }
    }

    fn scan_class(&mut self, class: &'a Class<'a>) {
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

    fn scan_expression(&mut self, expression: &'a Expression<'a>, context: ExprContext) {
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

    fn scan_argument(&mut self, argument: &'a Argument<'a>) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression, ExprContext::Other);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument, ExprContext::Other);
        }
    }

    fn scan_array_element(&mut self, element: &'a ArrayExpressionElement<'a>) {
        if let Some(expression) = element.as_expression() {
            self.scan_expression(expression, ExprContext::Other);
        } else if let ArrayExpressionElement::SpreadElement(spread) = element {
            self.scan_expression(&spread.argument, ExprContext::Other);
        }
    }

    fn scan_assignment_target(&mut self, target: &'a AssignmentTarget<'a>) {
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

    fn check_call_expression(&mut self, call: &'a CallExpression<'a>, context: ExprContext) {
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

    fn check_new_expression(&mut self, new_expression: &'a NewExpression<'a>) {
        self.check_prefer_static_regex_new(new_expression);
        self.check_prefer_date_now_new(new_expression);
    }

    fn check_static_member_expression(
        &mut self,
        member: &'a oxc_ast::ast::StaticMemberExpression<'a>,
        context: ExprContext,
    ) {
        self.check_filter_length_member(member, context);
    }

    fn check_computed_member_expression(
        &mut self,
        member: &'a oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        self.check_prefer_array_at(member);
    }

    fn check_binary_expression(&mut self, binary: &'a oxc_ast::ast::BinaryExpression<'a>) {
        self.check_prefer_includes_binary(binary);
        self.check_no_indexof_equality(binary);
        self.check_prefer_array_some_binary(binary);
    }

    fn check_unary_expression(
        &mut self,
        unary: &'a oxc_ast::ast::UnaryExpression<'a>,
        context: ExprContext,
    ) {
        self.check_prefer_includes_unary(unary);
        self.check_prefer_array_some_unary(unary);
        self.check_prefer_date_now_unary(unary);
        self.check_no_delete_property(unary, context);
    }

    fn check_ban_dependency_import(&mut self, import: &'a ImportDeclaration<'a>) {
        self.check_ban_dependency_source(import.source.value.as_str(), import.source.span);
    }

    fn check_ban_dependency_require(&mut self, call: &'a CallExpression<'a>) {
        if !self.options.has_rule("ban-dependencies") {
            return;
        }
        if !matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "require")
        {
            return;
        }
        let Some(Expression::StringLiteral(source)) = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .map(Expression::get_inner_expression)
        else {
            return;
        };
        self.check_ban_dependency_source(source.value.as_str(), source.span);
    }

    fn check_ban_dependency_source(&mut self, source: &str, span: Span) {
        if !self.options.has_rule("ban-dependencies") {
            return;
        }
        for dependency in &self.options.banned_dependencies {
            if source == dependency.module_name
                || source
                    .strip_prefix(dependency.module_name.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
            {
                let diagnostic =
                    ban_dependency_diagnostic(dependency, span, self.source_text, &self.line_index);
                self.diagnostics.push(diagnostic);
                return;
            }
        }
    }

    fn check_prefer_exponentiation(&mut self, call: &'a CallExpression<'a>) {
        if !is_static_call(call, "Math", "pow") || call.arguments.len() != 2 {
            return;
        }
        let Some(base) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Some(exponent) = call.arguments.get(1).and_then(Argument::as_expression) else {
            return;
        };
        let replacement = format!(
            "({}) ** ({})",
            self.text(base.span()),
            self.text(exponent.span())
        );
        self.report_with_fix(
            "prefer-exponentiation-operator",
            "preferExponentiation",
            call.span,
            replacement,
        );
    }

    fn check_prefer_object_has_own(&mut self, call: &'a CallExpression<'a>) {
        if call.arguments.len() == 2
            && callee_path(&call.callee).as_deref() == Some("Object.prototype.hasOwnProperty.call")
        {
            let object = call.arguments[0].span();
            let property = call.arguments[1].span();
            let replacement = format!(
                "Object.hasOwn({}, {})",
                self.text(object),
                self.text(property)
            );
            self.report_with_fix(
                "prefer-object-has-own",
                "preferObjectHasOwn",
                call.span,
                replacement,
            );
            return;
        }

        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "hasOwnProperty" || call.arguments.len() != 1 {
            return;
        }
        let replacement = format!(
            "Object.hasOwn({}, {})",
            self.text(object.span()),
            self.text(call.arguments[0].span())
        );
        self.report_with_fix(
            "prefer-object-has-own",
            "preferObjectHasOwn",
            call.span,
            replacement,
        );
    }

    fn check_prefer_array_from_map(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "map" || call.arguments.len() != 1 {
            return;
        }
        let Expression::ArrayExpression(array) = object.get_inner_expression() else {
            return;
        };
        let Some(spread) = single_spread_element(array) else {
            return;
        };
        let iterable = self.text(spread.argument.span());
        let mapper = self.text(call.arguments[0].span());
        let replacement = format!("Array.from({iterable}, {mapper})");
        self.report_with_data(
            "prefer-array-from-map",
            "preferArrayFrom",
            DiagnosticData {
                iterable: Some(CompactString::from(iterable)),
                mapper: Some(CompactString::from(mapper)),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }

    fn check_prefer_array_fill(&mut self, call: &'a CallExpression<'a>) {
        if is_static_call(call, "Array", "from") && call.arguments.len() == 2 {
            let Some(Expression::ObjectExpression(object)) = call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .map(Expression::get_inner_expression)
            else {
                return;
            };
            let Some(length_value) = object_length_value(object) else {
                return;
            };
            let Some(callback) = call.arguments.get(1).and_then(Argument::as_expression) else {
                return;
            };
            let Some(value) = constant_callback_value(callback) else {
                return;
            };
            let length_text = self.text(length_value.span());
            let value_text = self.text(value.span());
            let replacement = format!("Array.from({{length: {length_text}}}).fill({value_text})");
            self.report_with_data(
                "prefer-array-fill",
                "preferFillArrayFrom",
                DiagnosticData {
                    length: Some(CompactString::from(length_text)),
                    value: Some(CompactString::from(value_text)),
                    ..DiagnosticData::default()
                },
                call.span,
                Some(DiagnosticFix {
                    start: call.span.start,
                    end: call.span.end,
                    replacement: CompactString::from(replacement),
                }),
            );
            return;
        }

        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "map" || call.arguments.len() != 1 {
            return;
        }
        let Expression::ArrayExpression(array) = object.get_inner_expression() else {
            return;
        };
        let Some(spread) = single_spread_element(array) else {
            return;
        };
        let Expression::CallExpression(array_call) = spread.argument.get_inner_expression() else {
            return;
        };
        if !matches!(array_call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Array")
            || array_call.arguments.len() != 1
        {
            return;
        }
        let Some(callback) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Some(value) = constant_callback_value(callback) else {
            return;
        };
        let length_text = self.text(array_call.arguments[0].span());
        let value_text = self.text(value.span());
        let replacement = format!("Array({length_text}).fill({value_text})");
        self.report_with_data(
            "prefer-array-fill",
            "preferFillSpreadMap",
            DiagnosticData {
                length: Some(CompactString::from(length_text)),
                value: Some(CompactString::from(value_text)),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }

    fn check_prefer_spread_syntax(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };

        if property == "concat"
            && !matches!(object.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Buffer")
            && !call.arguments.is_empty()
        {
            let mut parts = SmallVec::<[CompactString; 8]>::new();
            if let Expression::ArrayExpression(array) = object.get_inner_expression() {
                for element in &array.elements {
                    if let Some(expression) = element.as_expression() {
                        parts.push(CompactString::from(self.text(expression.span())));
                    } else if let ArrayExpressionElement::SpreadElement(spread) = element {
                        parts.push(CompactString::from(self.text(spread.span)));
                    }
                }
            } else {
                parts.push(CompactString::from(format!(
                    "...{}",
                    self.text(object.span())
                )));
            }
            for argument in &call.arguments {
                if let Argument::SpreadElement(spread) = argument {
                    parts.push(CompactString::from(self.text(spread.span)));
                } else if let Some(Expression::ArrayExpression(array)) = argument
                    .as_expression()
                    .map(Expression::get_inner_expression)
                {
                    for element in &array.elements {
                        if let Some(expression) = element.as_expression() {
                            parts.push(CompactString::from(self.text(expression.span())));
                        }
                    }
                } else {
                    parts.push(CompactString::from(format!(
                        "...{}",
                        self.text(argument.span())
                    )));
                }
            }
            let replacement = format!("[{}]", parts.join(", "));
            self.report_with_fix(
                "prefer-spread-syntax",
                "preferSpreadArray",
                call.span,
                replacement,
            );
            return;
        }

        if is_static_call(call, "Array", "from") && call.arguments.len() == 1 {
            let Some(first_arg) = call.arguments.first() else {
                return;
            };
            if !matches!(first_arg, Argument::SpreadElement(_))
                && !matches!(
                    first_arg
                        .as_expression()
                        .map(Expression::get_inner_expression),
                    Some(Expression::ObjectExpression(_))
                )
            {
                let replacement = format!("[...{}]", self.text(first_arg.span()));
                self.report_with_fix(
                    "prefer-spread-syntax",
                    "preferSpreadArrayFrom",
                    call.span,
                    replacement,
                );
            }
            return;
        }

        if is_static_call(call, "Object", "assign") && call.arguments.len() >= 2 {
            let Some(Expression::ObjectExpression(first_object)) = call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .map(Expression::get_inner_expression)
            else {
                return;
            };
            if call
                .arguments
                .iter()
                .skip(1)
                .any(|arg| matches!(arg, Argument::SpreadElement(_)))
            {
                return;
            }
            let mut replacement = String::from("{");
            if !first_object.properties.is_empty() {
                let first_text = self.text(call.arguments[0].span());
                replacement.push_str(first_text.trim_start_matches('{').trim_end_matches('}'));
                replacement.push_str(", ");
            }
            let spreads: Vec<String> = call
                .arguments
                .iter()
                .skip(1)
                .map(|arg| format!("...{}", self.text(arg.span())))
                .collect();
            replacement.push_str(&spreads.join(", "));
            replacement.push('}');
            self.report_with_fix(
                "prefer-spread-syntax",
                "preferSpreadObject",
                call.span,
                replacement,
            );
            return;
        }

        if property == "apply" && call.arguments.len() == 2 {
            let Some(first_arg) = call.arguments.first().and_then(Argument::as_expression) else {
                return;
            };
            if !is_null_or_undefined(first_arg) {
                return;
            }
            let replacement = format!(
                "{}(...{})",
                self.text(object.span()),
                self.text(call.arguments[1].span())
            );
            self.report_with_fix(
                "prefer-spread-syntax",
                "preferSpreadFunction",
                call.span,
                replacement,
            );
        }
    }

    fn check_prefer_copy_method(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        let Some((rule_name, message_id, method)) = (match property {
            "reverse" => Some(("prefer-array-to-reversed", "preferToReversed", "toReversed")),
            "sort" => Some(("prefer-array-to-sorted", "preferToSorted", "toSorted")),
            "splice" => Some(("prefer-array-to-spliced", "preferToSpliced", "toSpliced")),
            _ => None,
        }) else {
            return;
        };
        let Some(array) = copy_pattern_source(object) else {
            return;
        };
        let raw_text = self.text(array.span());
        let args = call
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let access = if copy_pattern_optional(object) {
            "?."
        } else {
            "."
        };
        let replacement = format!("{raw_text}{access}{method}({args})");
        self.report_with_data(
            rule_name,
            message_id,
            DiagnosticData {
                array: Some(CompactString::from(raw_text)),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }

    fn check_prefer_array_at(&mut self, member: &'a oxc_ast::ast::ComputedMemberExpression<'a>) {
        let Expression::BinaryExpression(binary) = member.expression.get_inner_expression() else {
            return;
        };
        if binary.operator != BinaryOperator::Subtraction || !is_number_literal(&binary.right, 1.0)
        {
            return;
        }
        let Expression::StaticMemberExpression(length_member) = binary.left.get_inner_expression()
        else {
            return;
        };
        if length_member.property.name != "length" {
            return;
        }
        let array_text = self.text(member.object.span());
        if array_text != self.text(length_member.object.span()) {
            return;
        }
        let replacement = format!("{array_text}.at(-1)");
        self.report_with_data(
            "prefer-array-at",
            "preferAt",
            DiagnosticData {
                array: Some(CompactString::from(array_text)),
                ..DiagnosticData::default()
            },
            member.span,
            Some(DiagnosticFix {
                start: member.span.start,
                end: member.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }

    fn check_prefer_includes_binary(&mut self, binary: &'a oxc_ast::ast::BinaryExpression<'a>) {
        let Some((index_call, constant, reversed)) =
            binary_index_of_comparison(&binary.left, &binary.right)
        else {
            return;
        };
        let op = normalize_operator(binary.operator, reversed);
        let Some(should_negate) = includes_negation_for_constant(op, constant) else {
            return;
        };
        self.report_index_of_as_includes(binary.span, index_call, should_negate);
    }

    fn check_prefer_includes_unary(&mut self, unary: &'a oxc_ast::ast::UnaryExpression<'a>) {
        if unary.operator == UnaryOperator::BitwiseNot {
            if let Expression::CallExpression(call) = unary.argument.get_inner_expression() {
                if is_method_call(call, "indexOf") {
                    self.report_index_of_as_includes(unary.span, call, false);
                }
            }
        } else if unary.operator == UnaryOperator::LogicalNot {
            let Expression::UnaryExpression(inner) = unary.argument.get_inner_expression() else {
                return;
            };
            if inner.operator == UnaryOperator::BitwiseNot {
                if let Expression::CallExpression(call) = inner.argument.get_inner_expression() {
                    if is_method_call(call, "indexOf") {
                        self.report_index_of_as_includes(unary.span, call, true);
                    }
                }
            }
        }
    }

    fn report_index_of_as_includes(
        &mut self,
        span: Span,
        index_call: &'a CallExpression<'a>,
        should_negate: bool,
    ) {
        let Some((object, _)) = static_member_callee(index_call) else {
            return;
        };
        let args = index_call
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let replacement = if should_negate {
            format!("!{}.includes({args})", self.text(object.span()))
        } else {
            format!("{}.includes({args})", self.text(object.span()))
        };
        self.report_with_fix("prefer-includes", "preferIncludes", span, replacement);
    }

    fn check_no_indexof_equality(&mut self, binary: &'a oxc_ast::ast::BinaryExpression<'a>) {
        if !matches!(
            binary.operator,
            BinaryOperator::Equality | BinaryOperator::StrictEquality
        ) {
            return;
        }
        let Some((index_call, constant, _)) =
            binary_index_of_comparison(&binary.left, &binary.right)
        else {
            return;
        };
        let Some(index) = numeric_literal_value(constant) else {
            return;
        };
        if index < 0.0 || index.fract() != 0.0 {
            return;
        }
        let Some((object, _)) = static_member_callee(index_call) else {
            return;
        };
        let Some(search) = index_call.arguments.first() else {
            return;
        };
        let object_text = self.text(object.span());
        let search_text = self.text(search.span());
        if index == 0.0 {
            self.report_with_fix(
                "no-indexof-equality",
                "preferStartsWith",
                binary.span,
                format!("{object_text}.startsWith({search_text})"),
            );
        } else {
            let index_text = format!("{index:.0}");
            self.report_with_data(
                "no-indexof-equality",
                "preferDirectAccess",
                DiagnosticData {
                    array: Some(CompactString::from(object_text)),
                    item: Some(CompactString::from(search_text)),
                    index: Some(CompactString::from(index_text.clone())),
                    ..DiagnosticData::default()
                },
                binary.span,
                Some(DiagnosticFix {
                    start: binary.span.start,
                    end: binary.span.end,
                    replacement: CompactString::from(format!(
                        "{object_text}[{index_text}] === {search_text}"
                    )),
                }),
            );
        }
    }

    fn check_prefer_nullish_conditional(
        &mut self,
        conditional: &'a oxc_ast::ast::ConditionalExpression<'a>,
    ) {
        let Some(check) = nullish_check(self.source_text, &conditional.test) else {
            return;
        };
        let compare = if check.checks_for_nullish {
            &conditional.alternate
        } else {
            &conditional.consequent
        };
        let default = if check.checks_for_nullish {
            &conditional.consequent
        } else {
            &conditional.alternate
        };
        if self.text(check.value.span()) != self.text(compare.span()) {
            return;
        }
        let replacement = format!(
            "{} ?? {}",
            self.text(check.value.span()),
            self.text(default.span())
        );
        self.report_with_fix(
            "prefer-nullish-coalescing",
            "preferNullishCoalescing",
            conditional.span,
            replacement,
        );
    }

    fn check_prefer_nullish_assignment(&mut self, statement: &'a oxc_ast::ast::IfStatement<'a>) {
        if statement.alternate.is_some() {
            return;
        }
        let Some(check) = nullish_check(self.source_text, &statement.test) else {
            return;
        };
        if !check.checks_for_nullish {
            return;
        }
        let Some(expression_statement) = single_expression_statement(&statement.consequent) else {
            return;
        };
        let Expression::AssignmentExpression(assignment) =
            expression_statement.expression.get_inner_expression()
        else {
            return;
        };
        if assignment.operator != AssignmentOperator::Assign {
            return;
        }
        if self.text(check.value.span()) != self.text(assignment.left.span()) {
            return;
        }
        let replacement = format!(
            "{} ??= {}",
            self.text(assignment.left.span()),
            self.text(assignment.right.span())
        );
        self.report_with_fix(
            "prefer-nullish-coalescing",
            "preferNullishCoalescingAssignment",
            statement.span,
            replacement,
        );
    }

    fn check_prefer_date_now_call(&mut self, call: &'a CallExpression<'a>) {
        if is_static_call(call, "Date", "now") {
            return;
        }
        if is_static_call(call, "Number", "") && call.arguments.len() == 1 {
            let Some(Expression::NewExpression(new_date)) = call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .map(Expression::get_inner_expression)
            else {
                return;
            };
            if is_new_date_no_args(new_date) {
                self.report_with_fix("prefer-date-now", "preferDateNow", call.span, "Date.now()");
            }
            return;
        }
        if !is_method_call(call, "getTime") || !call.arguments.is_empty() {
            return;
        }
        let Some((object, _)) = static_member_callee(call) else {
            return;
        };
        if let Expression::NewExpression(new_date) = object.get_inner_expression() {
            if is_new_date_no_args(new_date) {
                self.report_with_fix("prefer-date-now", "preferDateNow", call.span, "Date.now()");
            }
        }
    }

    fn check_prefer_date_now_new(&mut self, _new_expression: &'a NewExpression<'a>) {}

    fn check_prefer_date_now_unary(&mut self, unary: &'a oxc_ast::ast::UnaryExpression<'a>) {
        if unary.operator != UnaryOperator::UnaryPlus {
            return;
        }
        let Expression::NewExpression(new_date) = unary.argument.get_inner_expression() else {
            return;
        };
        if is_new_date_no_args(new_date) {
            self.report_with_fix("prefer-date-now", "preferDateNow", unary.span, "Date.now()");
        }
    }

    fn check_prefer_regex_test(&mut self, call: &'a CallExpression<'a>, context: ExprContext) {
        if context != ExprContext::Boolean {
            return;
        }
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if call.arguments.len() != 1 {
            return;
        }
        let (regex, string) =
            if property == "match" && is_regex_expression(call.arguments[0].as_expression()) {
                (call.arguments[0].span(), object.span())
            } else if property == "exec" && is_regex_expression(Some(object)) {
                (object.span(), call.arguments[0].span())
            } else {
                return;
            };
        let regex_text = self.text(regex);
        let string_text = self.text(string);
        self.report_with_data(
            "prefer-regex-test",
            "preferTest",
            DiagnosticData {
                regex: Some(CompactString::from(regex_text)),
                string: Some(CompactString::from(string_text)),
                original: Some(CompactString::from(self.text(call.span))),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(format!("{regex_text}.test({string_text})")),
            }),
        );
    }

    fn check_prefer_static_regex_call(&mut self, call: &'a CallExpression<'a>) {
        if self.function_depth == 0
            || !matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "RegExp")
        {
            return;
        }
        if static_regexp_args(&call.arguments) {
            self.report("prefer-static-regex", "preferStatic", call.span);
        }
    }

    fn check_prefer_static_regex_new(&mut self, new_expression: &'a NewExpression<'a>) {
        if self.function_depth == 0
            || !matches!(new_expression.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "RegExp")
        {
            return;
        }
        if static_regexp_args(&new_expression.arguments) {
            self.report("prefer-static-regex", "preferStatic", new_expression.span);
        }
    }

    fn check_prefer_inline_equality(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "includes" || call.arguments.len() != 1 {
            return;
        }
        let Expression::ArrayExpression(array) = object.get_inner_expression() else {
            return;
        };
        if array.elements.is_empty() || array.elements.len() > 6 {
            return;
        }
        let value_text = self.text(call.arguments[0].span());
        let mut parts = Vec::new();
        for element in &array.elements {
            let Some(expression) = element.as_expression() else {
                return;
            };
            if !is_simple_inline_element(expression) {
                return;
            }
            parts.push(format!("{} === {value_text}", self.text(expression.span())));
        }
        self.report_with_fix(
            "prefer-inline-equality",
            "preferEquality",
            call.span,
            parts.join(" || "),
        );
    }

    fn check_prefer_string_from_char_code(&mut self, call: &'a CallExpression<'a>) {
        if !is_static_call(call, "String", "fromCodePoint") || call.arguments.is_empty() {
            return;
        }
        if !call.arguments.iter().all(is_safe_from_code_point_arg) {
            return;
        }
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        self.report_with_data(
            "prefer-string-fromcharcode",
            "preferFromCharCode",
            DiagnosticData::default(),
            member.property.span,
            Some(DiagnosticFix {
                start: member.property.span.start,
                end: member.property.span.end,
                replacement: CompactString::from("fromCharCode"),
            }),
        );
    }

    fn check_prefer_timer_args(&mut self, call: &'a CallExpression<'a>) {
        if !is_timer_call(call) || call.arguments.len() < 2 {
            return;
        }
        let Some(first_arg) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let delay_text = self.text(call.arguments[1].span());
        let timer_text = self.text(call.callee.span());
        let replacement = match first_arg.get_inner_expression() {
            Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
                if !arrow.expression {
                    return;
                };
                let Some(body) = expression_body(&arrow.body) else {
                    return;
                };
                let Expression::CallExpression(inner_call) = body.get_inner_expression() else {
                    return;
                };
                if matches!(
                    inner_call.callee.get_inner_expression(),
                    Expression::StaticMemberExpression(_)
                ) {
                    return;
                }
                let args = inner_call
                    .arguments
                    .iter()
                    .map(|argument| self.text(argument.span()).to_owned())
                    .collect::<Vec<_>>();
                format_timer_replacement(
                    &timer_text,
                    self.text(inner_call.callee.span()),
                    &delay_text,
                    &args,
                )
            }
            Expression::CallExpression(bind_call) if is_method_call(bind_call, "bind") => {
                let Some(bind_context) = bind_call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                else {
                    return;
                };
                if !is_null_or_undefined(bind_context) {
                    return;
                }
                let Some((fn_expression, _)) = static_member_callee(bind_call) else {
                    return;
                };
                let args = bind_call
                    .arguments
                    .iter()
                    .skip(1)
                    .map(|argument| self.text(argument.span()).to_owned())
                    .collect::<Vec<_>>();
                format_timer_replacement(
                    &timer_text,
                    self.text(fn_expression.span()),
                    &delay_text,
                    &args,
                )
            }
            _ => return,
        };
        self.report_with_fix("prefer-timer-args", "preferArgs", call.span, replacement);
    }

    fn check_prefer_includes_over_regex_test(&mut self, call: &'a CallExpression<'a>) {
        if !is_method_call(call, "test") || call.arguments.len() != 1 {
            return;
        }
        let Some((regex, _)) = static_member_callee(call) else {
            return;
        };
        let Some((message_id, replacement_method, value)) =
            simple_regex_equivalent(self.text(regex.span()))
        else {
            return;
        };
        let string_text = self.text(call.arguments[0].span());
        let replacement = if replacement_method == "===" {
            format!("{string_text} === {value:?}")
        } else {
            format!("{string_text}.{replacement_method}({value:?})")
        };
        self.report_with_fix(
            "prefer-includes-over-regex-test",
            message_id,
            call.span,
            replacement,
        );
    }

    fn check_no_spread_in_reduce(&mut self, call: &'a CallExpression<'a>) {
        let Some((_, property)) = static_member_callee(call) else {
            return;
        };
        if property != "reduce" {
            return;
        }
        for argument in &call.arguments {
            let Some(expression) = argument.as_expression() else {
                continue;
            };
            if function_body_contains_spread(expression) {
                self.report("no-spread-in-reduce", "noSpreadInReduce", expression.span());
            }
        }
    }

    fn check_prefer_static_collator(&mut self, call: &'a CallExpression<'a>) {
        if self.function_depth == 0 {
            return;
        }
        if is_static_call(call, "Intl", "Collator") {
            self.report("prefer-static-collator", "preferStaticCollator", call.span);
        }
    }

    fn check_prefer_array_some_binary(&mut self, binary: &'a oxc_ast::ast::BinaryExpression<'a>) {
        let Some((kind, constant, reversed)) =
            find_or_filter_comparison(&binary.left, &binary.right)
        else {
            return;
        };
        let op = normalize_operator(binary.operator, reversed);
        let should_negate = match kind {
            SomeSource::Find(call) if is_undefined_constant(constant) => match op {
                BinaryOperator::StrictEquality | BinaryOperator::Equality => Some(true),
                BinaryOperator::StrictInequality | BinaryOperator::Inequality => Some(false),
                _ => None,
            }
            .map(|negate| (call, negate)),
            SomeSource::FilterLength(call) => {
                let Some(value) = numeric_literal_value(constant) else {
                    return;
                };
                let negate = if value == 0.0 {
                    match op {
                        BinaryOperator::StrictEquality
                        | BinaryOperator::Equality
                        | BinaryOperator::LessEqualThan => Some(true),
                        BinaryOperator::StrictInequality
                        | BinaryOperator::Inequality
                        | BinaryOperator::GreaterThan => Some(false),
                        _ => None,
                    }
                } else if value == 1.0 {
                    match op {
                        BinaryOperator::LessThan => Some(true),
                        BinaryOperator::GreaterEqualThan => Some(false),
                        _ => None,
                    }
                } else {
                    None
                };
                let Some(negate) = negate else {
                    return;
                };
                Some((call, negate))
            }
            _ => None,
        };
        if let Some((call, negate)) = should_negate {
            self.report_array_some(binary.span, call, negate);
        }
    }

    fn check_prefer_array_some_unary(&mut self, unary: &'a oxc_ast::ast::UnaryExpression<'a>) {
        if unary.operator != UnaryOperator::LogicalNot {
            return;
        }
        if let Some(call) = find_call_or_filter_length(&unary.argument) {
            self.report_array_some(unary.span, call, true);
            return;
        }
        let Expression::UnaryExpression(inner) = unary.argument.get_inner_expression() else {
            return;
        };
        if inner.operator == UnaryOperator::LogicalNot {
            if let Some(call) = find_call_or_filter_length(&inner.argument) {
                self.report_array_some(unary.span, call, false);
            }
        }
    }

    fn check_prefer_array_some_call(&mut self, call: &'a CallExpression<'a>, context: ExprContext) {
        if context == ExprContext::Boolean && is_method_call(call, "find") {
            self.report_array_some(call.span, call, false);
        }
    }

    fn check_filter_length_member(
        &mut self,
        member: &'a oxc_ast::ast::StaticMemberExpression<'a>,
        context: ExprContext,
    ) {
        if context == ExprContext::Boolean && member.property.name == "length" {
            if let Expression::CallExpression(call) = member.object.get_inner_expression() {
                if is_method_call(call, "filter") {
                    self.report_array_some(member.span, call, false);
                }
            }
        }
    }

    fn report_array_some(&mut self, span: Span, call: &'a CallExpression<'a>, negate: bool) {
        let Some((object, _)) = static_member_callee(call) else {
            return;
        };
        let args = call
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let replacement = if negate {
            format!("!{}.some({args})", self.text(object.span()))
        } else {
            format!("{}.some({args})", self.text(object.span()))
        };
        self.report_with_fix("prefer-array-some", "preferArraySome", span, replacement);
    }

    fn check_prefer_url_canparse(&mut self, statement: &'a oxc_ast::ast::TryStatement<'a>) {
        let Some(handler) = &statement.handler else {
            return;
        };
        if statement.block.body.len() != 2 || handler.body.body.len() != 1 {
            return;
        }
        let Statement::ExpressionStatement(first) = &statement.block.body[0] else {
            return;
        };
        let Expression::NewExpression(new_url) = first.expression.get_inner_expression() else {
            return;
        };
        if !matches!(new_url.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "URL")
            || new_url.arguments.is_empty()
        {
            return;
        }
        let Statement::ReturnStatement(ok_return) = &statement.block.body[1] else {
            return;
        };
        let Statement::ReturnStatement(error_return) = &handler.body.body[0] else {
            return;
        };
        if !return_boolean(ok_return, true) || !return_boolean(error_return, false) {
            return;
        }
        let args = new_url
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        self.report_with_fix(
            "prefer-url-canparse",
            "preferCanParse",
            statement.span,
            format!("return URL.canParse({args})"),
        );
    }

    fn check_no_delete_property(
        &mut self,
        unary: &'a oxc_ast::ast::UnaryExpression<'a>,
        context: ExprContext,
    ) {
        if unary.operator != UnaryOperator::Delete {
            return;
        }
        let member_span = match unary.argument.get_inner_expression() {
            Expression::StaticMemberExpression(member) => Some(member.span),
            Expression::ComputedMemberExpression(member)
                if matches!(
                    member.expression.get_inner_expression(),
                    Expression::StringLiteral(_)
                ) =>
            {
                Some(member.span)
            }
            _ => None,
        };
        let Some(member_span) = member_span else {
            return;
        };
        let fix = if context == ExprContext::Statement {
            Some(DiagnosticFix {
                start: unary.span.start,
                end: unary.span.end,
                replacement: CompactString::from(format!("{} = undefined", self.text(member_span))),
            })
        } else {
            None
        };
        self.report_with_data(
            "no-delete-property",
            "noDeleteProperty",
            DiagnosticData::default(),
            unary.span,
            fix,
        );
    }

    fn text(&self, span: Span) -> &'a str {
        &self.source_text[span.start as usize..span.end as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExprContext {
    Boolean,
    Callee,
    MemberObject,
    Return,
    Statement,
    Other,
}

enum SomeSource<'a> {
    Find(&'a CallExpression<'a>),
    FilterLength(&'a CallExpression<'a>),
}

struct NullishCheck<'a> {
    value: &'a Expression<'a>,
    checks_for_nullish: bool,
}

fn ban_dependency_diagnostic(
    dependency: &BanDependency,
    span: Span,
    source_text: &str,
    line_index: &LineIndex,
) -> Diagnostic {
    let message_id = match dependency.message_id.as_str() {
        "nativeReplacement" => "nativeReplacement",
        "documentedReplacement" => "documentedReplacement",
        "simpleReplacement" => "simpleReplacement",
        "removalReplacement" => "removalReplacement",
        _ => "removalReplacement",
    };
    Diagnostic {
        rule_name: "ban-dependencies",
        message_id,
        data: DiagnosticData {
            name: Some(dependency.module_name.clone()),
            replacement: dependency.replacement.clone(),
            url: dependency.url.clone(),
            description: dependency.description.clone(),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(source_text, span),
        fix: None,
    }
}

fn static_member_callee<'a>(call: &'a CallExpression<'a>) -> Option<(&'a Expression<'a>, &'a str)> {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return None;
    };
    Some((&member.object, member.property.name.as_str()))
}

fn is_static_call(call: &CallExpression<'_>, object_name: &str, property_name: &str) -> bool {
    let Some((object, property)) = static_member_callee(call) else {
        return false;
    };
    if !property_name.is_empty() && property != property_name {
        return false;
    }
    matches!(object.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == object_name)
}

fn is_method_call(call: &CallExpression<'_>, method_name: &str) -> bool {
    static_member_callee(call).is_some_and(|(_, property)| property == method_name)
}

fn callee_path(expression: &Expression<'_>) -> Option<String> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.to_string()),
        Expression::StaticMemberExpression(member) => {
            let mut path = callee_path(&member.object)?;
            path.push('.');
            path.push_str(member.property.name.as_str());
            Some(path)
        }
        _ => None,
    }
}

fn single_spread_element<'a>(
    array: &'a ArrayExpression<'a>,
) -> Option<&'a oxc_ast::ast::SpreadElement<'a>> {
    if array.elements.len() != 1 {
        return None;
    }
    let ArrayExpressionElement::SpreadElement(spread) = &array.elements[0] else {
        return None;
    };
    Some(spread)
}

fn object_length_value<'a>(
    object: &'a oxc_ast::ast::ObjectExpression<'a>,
) -> Option<&'a Expression<'a>> {
    if object.properties.len() != 1 {
        return None;
    }
    let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        return None;
    };
    if property_key_name(&property.key) != Some("length") {
        return None;
    }
    Some(&property.value)
}

fn expression_body<'a>(body: &'a FunctionBody<'a>) -> Option<&'a Expression<'a>> {
    if body.statements.len() != 1 {
        return None;
    }
    let Statement::ExpressionStatement(statement) = &body.statements[0] else {
        return None;
    };
    Some(&statement.expression)
}

fn property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

fn constant_callback_value<'a>(expression: &'a Expression<'a>) -> Option<&'a Expression<'a>> {
    match expression.get_inner_expression() {
        Expression::ArrowFunctionExpression(function) if function.params.items.is_empty() => {
            if function.expression {
                let expression = expression_body(&function.body)?;
                return is_constant_expression(expression).then_some(expression);
            }
            if function.body.statements.len() == 1 {
                let Statement::ReturnStatement(statement) = &function.body.statements[0] else {
                    return None;
                };
                let argument = statement.argument.as_ref()?;
                is_constant_expression(argument).then_some(argument)
            } else {
                None
            }
        }
        Expression::FunctionExpression(function) if function.params.items.is_empty() => {
            let body = function.body.as_ref()?;
            if body.statements.len() != 1 {
                return None;
            }
            let Statement::ReturnStatement(statement) = &body.statements[0] else {
                return None;
            };
            let argument = statement.argument.as_ref()?;
            is_constant_expression(argument).then_some(argument)
        }
        _ => None,
    }
}

fn is_constant_expression(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::Identifier(_)
        | Expression::RegExpLiteral(_) => true,
        Expression::StaticMemberExpression(member) => is_constant_expression(&member.object),
        Expression::ComputedMemberExpression(member) => {
            is_constant_expression(&member.object) && is_constant_expression(&member.expression)
        }
        Expression::UnaryExpression(unary) => is_constant_expression(&unary.argument),
        Expression::BinaryExpression(binary) => {
            is_constant_expression(&binary.left) && is_constant_expression(&binary.right)
        }
        Expression::LogicalExpression(logical) => {
            is_constant_expression(&logical.left) && is_constant_expression(&logical.right)
        }
        Expression::ConditionalExpression(conditional) => {
            is_constant_expression(&conditional.test)
                && is_constant_expression(&conditional.consequent)
                && is_constant_expression(&conditional.alternate)
        }
        Expression::TemplateLiteral(template) => {
            template.expressions.iter().all(is_constant_expression)
        }
        _ => false,
    }
}

fn copy_pattern_source<'a>(expression: &'a Expression<'a>) -> Option<&'a Expression<'a>> {
    match expression.get_inner_expression() {
        Expression::ArrayExpression(array) => {
            single_spread_element(array).map(|spread| &spread.argument)
        }
        Expression::CallExpression(call) if call.arguments.len() == 1 => {
            let Some((object, property)) = static_member_callee(call) else {
                if matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Array")
                {
                    return call.arguments.first().and_then(Argument::as_expression);
                }
                return None;
            };
            if property == "slice" && call.arguments.is_empty() {
                Some(object)
            } else if matches!(object.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Array")
                && property == "from"
            {
                call.arguments.first().and_then(Argument::as_expression)
            } else {
                None
            }
        }
        Expression::CallExpression(call) if call.arguments.is_empty() => {
            let Some((object, property)) = static_member_callee(call) else {
                return None;
            };
            (property == "slice").then_some(object)
        }
        _ => None,
    }
}

fn copy_pattern_optional(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::StaticMemberExpression(member) => member.optional,
        Expression::CallExpression(call) => call.optional,
        _ => false,
    }
}

fn is_number_literal(expression: &Expression<'_>, expected: f64) -> bool {
    numeric_literal_value(expression).is_some_and(|value| value == expected)
}

fn numeric_literal_value(expression: &Expression<'_>) -> Option<f64> {
    match expression.get_inner_expression() {
        Expression::NumericLiteral(literal) => Some(literal.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::UnaryNegation => {
            numeric_literal_value(&unary.argument).map(|value| -value)
        }
        _ => None,
    }
}

fn binary_index_of_comparison<'a>(
    left: &'a Expression<'a>,
    right: &'a Expression<'a>,
) -> Option<(&'a CallExpression<'a>, &'a Expression<'a>, bool)> {
    if let Expression::CallExpression(call) = left.get_inner_expression() {
        if is_method_call(call, "indexOf") {
            return Some((call, right, false));
        }
    }
    if let Expression::CallExpression(call) = right.get_inner_expression() {
        if is_method_call(call, "indexOf") {
            return Some((call, left, true));
        }
    }
    None
}

fn normalize_operator(operator: BinaryOperator, reversed: bool) -> BinaryOperator {
    if !reversed {
        return operator;
    }
    match operator {
        BinaryOperator::LessThan => BinaryOperator::GreaterThan,
        BinaryOperator::LessEqualThan => BinaryOperator::GreaterEqualThan,
        BinaryOperator::GreaterThan => BinaryOperator::LessThan,
        BinaryOperator::GreaterEqualThan => BinaryOperator::LessEqualThan,
        other => other,
    }
}

fn includes_negation_for_constant(
    operator: BinaryOperator,
    constant: &Expression<'_>,
) -> Option<bool> {
    if is_number_literal(constant, -1.0) {
        return match operator {
            BinaryOperator::StrictInequality
            | BinaryOperator::Inequality
            | BinaryOperator::GreaterThan => Some(false),
            BinaryOperator::StrictEquality | BinaryOperator::Equality => Some(true),
            _ => None,
        };
    }
    if is_number_literal(constant, 0.0) {
        return match operator {
            BinaryOperator::GreaterEqualThan => Some(false),
            BinaryOperator::LessThan => Some(true),
            _ => None,
        };
    }
    None
}

fn nullish_check<'a>(
    source_text: &str,
    expression: &'a Expression<'a>,
) -> Option<NullishCheck<'a>> {
    match expression.get_inner_expression() {
        Expression::BinaryExpression(binary)
            if matches!(
                binary.operator,
                BinaryOperator::Equality | BinaryOperator::Inequality
            ) && is_null_literal(&binary.right) =>
        {
            Some(NullishCheck {
                value: &binary.left,
                checks_for_nullish: binary.operator == BinaryOperator::Equality,
            })
        }
        Expression::LogicalExpression(logical) => {
            let Expression::BinaryExpression(left) = logical.left.get_inner_expression() else {
                return None;
            };
            let Expression::BinaryExpression(right) = logical.right.get_inner_expression() else {
                return None;
            };
            if source_text[left.left.span().start as usize..left.left.span().end as usize]
                != source_text[right.left.span().start as usize..right.left.span().end as usize]
            {
                return None;
            }
            let pair = (
                is_null_literal(&left.right),
                is_undefined_identifier(&left.right),
                is_null_literal(&right.right),
                is_undefined_identifier(&right.right),
            );
            if !matches!(
                pair,
                (true, false, false, true) | (false, true, true, false)
            ) {
                return None;
            }
            if logical.operator == LogicalOperator::Or
                && left.operator == BinaryOperator::StrictEquality
                && right.operator == BinaryOperator::StrictEquality
            {
                return Some(NullishCheck {
                    value: &left.left,
                    checks_for_nullish: true,
                });
            }
            if logical.operator == LogicalOperator::And
                && left.operator == BinaryOperator::StrictInequality
                && right.operator == BinaryOperator::StrictInequality
            {
                return Some(NullishCheck {
                    value: &left.left,
                    checks_for_nullish: false,
                });
            }
            None
        }
        _ => None,
    }
}

fn is_null_literal(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::NullLiteral(_)
    )
}

fn is_undefined_identifier(expression: &Expression<'_>) -> bool {
    matches!(expression.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "undefined")
}

fn is_null_or_undefined(expression: &Expression<'_>) -> bool {
    is_null_literal(expression) || is_undefined_identifier(expression)
}

fn single_expression_statement<'a>(
    statement: &'a Statement<'a>,
) -> Option<&'a oxc_ast::ast::ExpressionStatement<'a>> {
    match statement {
        Statement::ExpressionStatement(expression) => Some(expression),
        Statement::BlockStatement(block) if block.body.len() == 1 => {
            let Statement::ExpressionStatement(expression) = &block.body[0] else {
                return None;
            };
            Some(expression)
        }
        _ => None,
    }
}

fn is_new_date_no_args(new_expression: &NewExpression<'_>) -> bool {
    new_expression.arguments.is_empty()
        && matches!(new_expression.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Date")
}

fn is_regex_expression(expression: Option<&Expression<'_>>) -> bool {
    matches!(
        expression.map(Expression::get_inner_expression),
        Some(Expression::RegExpLiteral(_) | Expression::NewExpression(_))
    )
}

fn static_regexp_args(arguments: &[Argument<'_>]) -> bool {
    if arguments.is_empty() || arguments.len() > 2 {
        return false;
    }
    arguments.iter().all(|argument| {
        matches!(
            argument
                .as_expression()
                .map(Expression::get_inner_expression),
            Some(Expression::StringLiteral(_))
        )
    }) && arguments.get(1).is_none_or(|argument| {
        !argument
            .as_expression()
            .and_then(|expression| match expression.get_inner_expression() {
                Expression::StringLiteral(literal) => Some(literal.value.as_str()),
                _ => None,
            })
            .is_some_and(|flags| flags.contains('g') || flags.contains('y'))
    })
}

fn is_simple_inline_element(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::Identifier(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
}

fn is_safe_from_code_point_arg(argument: &Argument<'_>) -> bool {
    let Some(Expression::NumericLiteral(literal)) = argument
        .as_expression()
        .map(Expression::get_inner_expression)
    else {
        return false;
    };
    literal.value.fract() == 0.0 && (0.0..65536.0).contains(&literal.value)
}

fn is_timer_call(call: &CallExpression<'_>) -> bool {
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "setTimeout" | "setInterval")
        }
        Expression::StaticMemberExpression(member)
            if matches!(
                member.object.get_inner_expression(),
                Expression::Identifier(identifier)
                    if matches!(identifier.name.as_str(), "window" | "globalThis")
            ) =>
        {
            matches!(member.property.name.as_str(), "setTimeout" | "setInterval")
        }
        _ => false,
    }
}

fn format_timer_replacement(timer: &str, callee: &str, delay: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("{timer}({callee}, {delay})")
    } else {
        format!("{timer}({callee}, {delay}, {})", args.join(", "))
    }
}

fn simple_regex_equivalent(regex_text: &str) -> Option<(&'static str, &'static str, String)> {
    let inner = regex_text.strip_prefix('/')?;
    let pattern_end = inner.rfind('/')?;
    let (pattern, flags) = inner.split_at(pattern_end);
    let flags = &flags[1..];
    if flags.contains('i') || flags.contains('g') || flags.contains('y') || flags.contains('m') {
        return None;
    }
    if let Some(value) = pattern
        .strip_prefix('^')
        .and_then(|value| value.strip_suffix('$'))
    {
        if is_plain_regex_text(value) {
            return Some(("preferEquals", "===", value.to_owned()));
        }
    }
    if let Some(value) = pattern.strip_prefix('^') {
        if is_plain_regex_text(value) {
            return Some(("preferStartsWith", "startsWith", value.to_owned()));
        }
    }
    if let Some(value) = pattern.strip_suffix('$') {
        if is_plain_regex_text(value) {
            return Some(("preferEndsWith", "endsWith", value.to_owned()));
        }
    }
    if is_plain_regex_text(pattern) {
        return Some(("preferIncludes", "includes", pattern.to_owned()));
    }
    None
}

fn is_plain_regex_text(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(|ch| {
            matches!(
                ch,
                '.' | '*' | '+' | '?' | '[' | ']' | '(' | ')' | '{' | '}' | '|' | '\\'
            )
        })
}

fn function_body_contains_spread(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::ArrowFunctionExpression(function) => match &function.body {
            body if function.expression => {
                expression_body(body).is_some_and(expression_contains_spread)
            }
            body => body.statements.iter().any(statement_contains_spread),
        },
        Expression::FunctionExpression(function) => function
            .body
            .as_ref()
            .is_some_and(|body| body.statements.iter().any(statement_contains_spread)),
        _ => false,
    }
}

fn statement_contains_spread(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ReturnStatement(statement) => statement
            .argument
            .as_ref()
            .is_some_and(expression_contains_spread),
        Statement::ExpressionStatement(statement) => {
            expression_contains_spread(&statement.expression)
        }
        Statement::BlockStatement(block) => block.body.iter().any(statement_contains_spread),
        _ => false,
    }
}

fn expression_contains_spread(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::ObjectExpression(object) => object.properties.iter().any(|property| {
            matches!(property, ObjectPropertyKind::SpreadProperty(_))
                || matches!(property, ObjectPropertyKind::ObjectProperty(property) if expression_contains_spread(&property.value))
        }),
        Expression::ArrayExpression(array) => array.elements.iter().any(|element| {
            matches!(element, ArrayExpressionElement::SpreadElement(_))
                || element.as_expression().is_some_and(expression_contains_spread)
        }),
        Expression::CallExpression(call) => call
            .arguments
            .iter()
            .any(|argument| matches!(argument, Argument::SpreadElement(_))),
        _ => false,
    }
}

fn find_or_filter_comparison<'a>(
    left: &'a Expression<'a>,
    right: &'a Expression<'a>,
) -> Option<(SomeSource<'a>, &'a Expression<'a>, bool)> {
    if let Some(call) = find_call_or_filter_length(left) {
        return if is_method_call(call, "find") {
            Some((SomeSource::Find(call), right, false))
        } else {
            Some((SomeSource::FilterLength(call), right, false))
        };
    }
    if let Some(call) = find_call_or_filter_length(right) {
        return if is_method_call(call, "find") {
            Some((SomeSource::Find(call), left, true))
        } else {
            Some((SomeSource::FilterLength(call), left, true))
        };
    }
    None
}

fn find_call_or_filter_length<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a CallExpression<'a>> {
    match expression.get_inner_expression() {
        Expression::CallExpression(call) if is_method_call(call, "find") => Some(call),
        Expression::StaticMemberExpression(member) if member.property.name == "length" => {
            let Expression::CallExpression(call) = member.object.get_inner_expression() else {
                return None;
            };
            is_method_call(call, "filter").then_some(call)
        }
        _ => None,
    }
}

fn is_undefined_constant(expression: &Expression<'_>) -> bool {
    is_undefined_identifier(expression)
        || matches!(expression.get_inner_expression(), Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void)
}

fn return_boolean(statement: &oxc_ast::ast::ReturnStatement<'_>, value: bool) -> bool {
    matches!(
        statement.argument.as_ref().map(Expression::get_inner_expression),
        Some(Expression::BooleanLiteral(literal)) if literal.value == value
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(rule: &str, source: &str) -> SmallVec<[Diagnostic; 32]> {
        scan_e18e(
            source,
            "sample.ts",
            &E18eOptions {
                rule_names: [CompactString::from(rule)].into_iter().collect(),
                banned_dependencies: SmallVec::new(),
            },
        )
    }

    #[test]
    fn modern_array_rules_report_and_fix() {
        let diagnostics = scan(
            "prefer-array-from-map",
            "const out = [...items].map(item => item.id);",
        );
        assert_eq!(diagnostics[0].message_id, "preferArrayFrom");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "Array.from(items, item => item.id)"
        );

        let diagnostics = scan("prefer-array-at", "const last = items[items.length - 1];");
        assert_eq!(diagnostics[0].message_id, "preferAt");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "items.at(-1)"
        );
    }

    #[test]
    fn performance_rules_report_and_fix() {
        let diagnostics = scan(
            "prefer-exponentiation-operator",
            "const x = Math.pow(a, 2);",
        );
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "(a) ** (2)"
        );

        let diagnostics = scan(
            "prefer-string-fromcharcode",
            "String.fromCodePoint(65, 66);",
        );
        assert_eq!(diagnostics[0].loc.start_column, 7);
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "fromCharCode"
        );
    }

    #[test]
    fn boolean_rules_report_and_fix() {
        let diagnostics = scan("prefer-includes", "if (items.indexOf(id) !== -1) ok();");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "items.includes(id)"
        );

        let diagnostics = scan(
            "prefer-array-some",
            "if (items.filter(fn).length > 0) ok();",
        );
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "items.some(fn)"
        );
    }

    #[test]
    fn ban_dependencies_uses_options() {
        let diagnostics = scan_e18e(
            "import merge from 'lodash.merge';",
            "sample.js",
            &E18eOptions {
                rule_names: [CompactString::from("ban-dependencies")]
                    .into_iter()
                    .collect(),
                banned_dependencies: [BanDependency {
                    module_name: CompactString::from("lodash.merge"),
                    message_id: CompactString::from("documentedReplacement"),
                    replacement: Some(CompactString::from("deepmerge-ts")),
                    url: Some(CompactString::from("https://example.com")),
                    description: None,
                }]
                .into_iter()
                .collect(),
            },
        );
        assert_eq!(diagnostics[0].message_id, "documentedReplacement");
        assert_eq!(diagnostics[0].data.name.as_deref(), Some("lodash.merge"));
    }
}
