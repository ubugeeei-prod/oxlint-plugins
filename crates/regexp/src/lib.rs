#![doc = "Rust implementation of selected eslint-plugin-regexp rule logic."]

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, AssignmentTarget, CallExpression, ChainElement, Class,
    ClassElement, Declaration, ExportDefaultDeclarationKind, Expression, ForStatementInit,
    ForStatementLeft, Function, FunctionBody, NewExpression, ObjectPropertyKind, PropertyKey,
    RegExpLiteral, Statement, VariableDeclaration,
};
use oxc_parser::Parser;
use oxc_regular_expression::{ConstructorParser, Options as RegExpOptions};
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub const RULE_NAMES: [&str; 10] = [
    "no-invalid-regexp",
    "no-empty-character-class",
    "no-empty-group",
    "no-empty-capturing-group",
    "no-empty-alternative",
    "no-zero-quantifier",
    "no-octal",
    "no-control-character",
    "sort-flags",
    "require-unicode-regexp",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub message: Option<CompactString>,
    pub flag: Option<CompactString>,
    pub flags: Option<CompactString>,
    pub sorted_flags: Option<CompactString>,
    pub expr: Option<CompactString>,
    pub char_text: Option<CompactString>,
}

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
    pub message_id: &'static str,
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
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

pub fn implemented_regexp_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_regexp(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 16]> {
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
    };
    scanner.scan_program(&parser_return.program.body);
    scanner.diagnostics
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 16]>,
}

impl<'a> Scanner<'a> {
    fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span);
    }

    fn report_with_data(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: DiagnosticData,
        span: Span,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }

    fn scan_program(&mut self, body: &'a [Statement<'a>]) {
        for statement in body {
            self.scan_statement(statement);
        }
    }

    fn scan_statement(&mut self, statement: &'a Statement<'a>) {
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

    fn scan_function(&mut self, function: &'a Function<'a>) {
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

    fn scan_class(&mut self, class: &'a Class<'a>) {
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

    fn scan_expression(&mut self, expression: &'a Expression<'a>) {
        match expression.get_inner_expression() {
            Expression::RegExpLiteral(literal) => self.check_regexp_literal(literal),
            Expression::CallExpression(call) => {
                self.check_call_expression(call);
                self.scan_expression(&call.callee);
                for argument in &call.arguments {
                    self.scan_argument(argument);
                }
            }
            Expression::NewExpression(new_expression) => {
                self.check_new_expression(new_expression);
                self.scan_expression(&new_expression.callee);
                for argument in &new_expression.arguments {
                    self.scan_argument(argument);
                }
            }
            Expression::AssignmentExpression(assignment) => {
                self.scan_assignment_target(&assignment.left);
                self.scan_expression(&assignment.right);
            }
            Expression::StaticMemberExpression(member) => self.scan_expression(&member.object),
            Expression::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object);
                self.scan_expression(&member.expression);
            }
            Expression::BinaryExpression(binary) => {
                self.scan_expression(&binary.left);
                self.scan_expression(&binary.right);
            }
            Expression::LogicalExpression(logical) => {
                self.scan_expression(&logical.left);
                self.scan_expression(&logical.right);
            }
            Expression::ConditionalExpression(conditional) => {
                self.scan_expression(&conditional.test);
                self.scan_expression(&conditional.consequent);
                self.scan_expression(&conditional.alternate);
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(expression) = array_element_expression(element) {
                        self.scan_expression(expression);
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed {
                                self.scan_property_key(&property.key);
                            }
                            self.scan_expression(&property.value);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(&spread.argument);
                        }
                    }
                }
            }
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.scan_expression(&tagged.tag);
                for expression in &tagged.quasi.expressions {
                    self.scan_expression(expression);
                }
            }
            Expression::FunctionExpression(function) => self.scan_function(function),
            Expression::ArrowFunctionExpression(function) => {
                for param in &function.params.items {
                    if let Some(initializer) = &param.initializer {
                        self.scan_expression(initializer);
                    }
                }
                for statement in &function.body.statements {
                    self.scan_statement(statement);
                }
            }
            Expression::ClassExpression(class) => self.scan_class(class),
            Expression::SequenceExpression(sequence) => {
                for expression in &sequence.expressions {
                    self.scan_expression(expression);
                }
            }
            Expression::AwaitExpression(await_expression) => {
                self.scan_expression(&await_expression.argument);
            }
            Expression::UnaryExpression(unary) => self.scan_expression(&unary.argument),
            Expression::UpdateExpression(_) => {}
            Expression::YieldExpression(yield_expression) => {
                if let Some(argument) = &yield_expression.argument {
                    self.scan_expression(argument);
                }
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    self.check_call_expression(call);
                    self.scan_expression(&call.callee);
                    for argument in &call.arguments {
                        self.scan_argument(argument);
                    }
                }
                ChainElement::StaticMemberExpression(member) => {
                    self.scan_expression(&member.object);
                }
                ChainElement::ComputedMemberExpression(member) => {
                    self.scan_expression(&member.object);
                    self.scan_expression(&member.expression);
                }
                ChainElement::PrivateFieldExpression(member) => {
                    self.scan_expression(&member.object);
                }
                ChainElement::TSNonNullExpression(expression) => {
                    self.scan_expression(&expression.expression);
                }
            },
            _ => {}
        }
    }

    fn scan_property_key(&mut self, key: &'a PropertyKey<'a>) {
        if let Some(expression) = key.as_expression() {
            self.scan_expression(expression);
        }
    }

    fn scan_argument(&mut self, argument: &'a Argument<'a>) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument);
        }
    }

    fn scan_assignment_target(&mut self, target: &'a AssignmentTarget<'a>) {
        match target {
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object);
                self.scan_expression(&member.expression);
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                self.scan_expression(&member.object);
            }
            AssignmentTarget::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object);
            }
            AssignmentTarget::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression)
            }
            AssignmentTarget::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression);
            }
            AssignmentTarget::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression);
            }
            AssignmentTarget::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression);
            }
            _ => {}
        }
    }

    fn check_call_expression(&mut self, call: &'a CallExpression<'a>) {
        if call.callee.is_specific_id("RegExp") {
            self.check_regexp_constructor(call.span, &call.arguments);
        }
    }

    fn check_new_expression(&mut self, new_expression: &'a NewExpression<'a>) {
        if new_expression.callee.is_specific_id("RegExp") {
            self.check_regexp_constructor(new_expression.span, &new_expression.arguments);
        }
    }

    fn check_regexp_literal(&mut self, literal: &'a RegExpLiteral<'a>) {
        let pattern = literal.regex.pattern.text.as_str();
        let flags = literal
            .raw
            .as_ref()
            .and_then(|raw| raw.as_str().rsplit_once('/').map(|(_, flags)| flags))
            .unwrap_or("");
        self.check_regexp(pattern, flags, literal.span, false, None, None);
    }

    fn check_regexp_constructor(
        &mut self,
        span: Span,
        arguments: &'a oxc_allocator::Vec<'a, Argument<'a>>,
    ) {
        let Some(pattern_argument) = arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Some((pattern, pattern_span)) = string_literal_value_with_span(pattern_argument) else {
            return;
        };
        let flags = arguments
            .get(1)
            .and_then(Argument::as_expression)
            .and_then(string_literal_value_with_span);
        let flags_value = flags.map_or("", |(value, _)| value);
        self.check_regexp(
            pattern,
            flags_value,
            span,
            true,
            Some(pattern_span),
            flags.map(|(_, span)| span),
        );
    }

    fn check_regexp(
        &mut self,
        pattern: &str,
        flags: &str,
        span: Span,
        is_constructor: bool,
        pattern_span: Option<Span>,
        flags_span: Option<Span>,
    ) {
        if let Some(flag) = duplicate_flag(flags) {
            self.report_with_data(
                "no-invalid-regexp",
                "duplicateFlag",
                DiagnosticData {
                    flag: Some(CompactString::from(flag)),
                    ..DiagnosticData::default()
                },
                span,
            );
            return;
        }
        if flags.contains('u') && flags.contains('v') {
            self.report("no-invalid-regexp", "uvFlag", span);
            return;
        }
        if let (true, Some(message)) = (
            is_constructor,
            self.constructor_parse_error(pattern_span, flags_span),
        ) {
            self.report_with_data(
                "no-invalid-regexp",
                "error",
                DiagnosticData {
                    message: Some(message),
                    ..DiagnosticData::default()
                },
                span,
            );
            return;
        }

        self.check_flag_style(flags, span);
        self.check_pattern_rules(pattern, span);
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "Oxc regexp parser exposes display text; this allocation is only diagnostic data."
    )]
    fn constructor_parse_error(
        &self,
        pattern_span: Option<Span>,
        flags_span: Option<Span>,
    ) -> Option<CompactString> {
        let pattern_span = pattern_span?;
        let allocator = Allocator::default();
        let parsed = ConstructorParser::new(
            &allocator,
            pattern_span.source_text(self.source_text),
            flags_span.map(|span| span.source_text(self.source_text)),
            RegExpOptions {
                pattern_span_offset: pattern_span.start,
                flags_span_offset: flags_span.map_or(0, |span| span.start),
            },
        )
        .parse();
        match parsed {
            Ok(_) => None,
            Err(error) => Some(CompactString::from(error.to_string().as_str())),
        }
    }

    fn check_flag_style(&mut self, flags: &str, span: Span) {
        let sorted_flags = sorted_flags(flags);
        if flags != sorted_flags.as_str() {
            self.report_with_data(
                "sort-flags",
                "sortFlags",
                DiagnosticData {
                    flags: Some(CompactString::from(flags)),
                    sorted_flags: Some(sorted_flags),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if !flags.contains('u') && !flags.contains('v') {
            self.report("require-unicode-regexp", "require", span);
        }
    }

    fn check_pattern_rules(&mut self, pattern: &str, span: Span) {
        let mut analysis = PatternAnalysis::new();
        analysis.scan(pattern);

        if analysis.has_empty_character_class {
            self.report("no-empty-character-class", "empty", span);
        }
        if analysis.has_empty_group {
            self.report("no-empty-group", "unexpected", span);
        }
        if analysis.has_empty_capturing_group {
            self.report("no-empty-capturing-group", "unexpected", span);
        }
        if analysis.has_empty_alternative {
            self.report("no-empty-alternative", "empty", span);
        }
        if analysis.has_zero_quantifier {
            self.report("no-zero-quantifier", "unexpected", span);
        }
        if let Some(expr) = first_octal_escape(pattern) {
            self.report_with_data(
                "no-octal",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(expr)),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(ch) = first_control_character(pattern) {
            self.report_with_data(
                "no-control-character",
                "unexpected",
                DiagnosticData {
                    char_text: Some(mention_char(ch)),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
    }
}

#[derive(Clone, Copy)]
struct GroupState {
    check_empty: bool,
    capturing: bool,
    seen_pipe: bool,
    current_has_content: bool,
}

impl GroupState {
    fn top_level() -> Self {
        Self {
            check_empty: false,
            capturing: false,
            seen_pipe: false,
            current_has_content: false,
        }
    }

    fn group(check_empty: bool, capturing: bool) -> Self {
        Self {
            check_empty,
            capturing,
            seen_pipe: false,
            current_has_content: false,
        }
    }
}

#[derive(Default)]
struct PatternAnalysis {
    has_empty_character_class: bool,
    has_empty_group: bool,
    has_empty_capturing_group: bool,
    has_empty_alternative: bool,
    has_zero_quantifier: bool,
}

impl PatternAnalysis {
    fn new() -> Self {
        Self::default()
    }

    fn scan(&mut self, pattern: &str) {
        let bytes = pattern.as_bytes();
        let mut groups = SmallVec::<[GroupState; 8]>::new();
        groups.push(GroupState::top_level());
        let mut index = 0;

        while index < bytes.len() {
            match bytes[index] {
                b'\\' => {
                    self.mark_content(&mut groups);
                    index = skip_escape(bytes, index);
                }
                b'[' => {
                    let close = find_class_end(bytes, index);
                    if let Some(close) = close {
                        if close == index + 1 {
                            self.has_empty_character_class = true;
                        }
                        self.mark_content(&mut groups);
                        index = close + 1;
                    } else {
                        self.mark_content(&mut groups);
                        index += 1;
                    }
                }
                b'(' => {
                    let (check_empty, capturing, next) = group_prefix(bytes, index);
                    groups.push(GroupState::group(check_empty, capturing));
                    index = next;
                }
                b')' => {
                    if groups.len() > 1
                        && let Some(group) = groups.pop()
                    {
                        if group.seen_pipe && !group.current_has_content {
                            self.has_empty_alternative = true;
                        }
                        if group.check_empty && !group.seen_pipe && !group.current_has_content {
                            self.has_empty_group = true;
                            if group.capturing {
                                self.has_empty_capturing_group = true;
                            }
                        }
                        self.mark_content(&mut groups);
                    }
                    index += 1;
                }
                b'|' => {
                    if let Some(group) = groups.last_mut() {
                        if !group.current_has_content {
                            self.has_empty_alternative = true;
                        }
                        group.seen_pipe = true;
                        group.current_has_content = false;
                    }
                    index += 1;
                }
                b'{' if is_zero_quantifier(bytes, index) => {
                    self.has_zero_quantifier = true;
                    index += 1;
                }
                b'*' | b'+' | b'?' | b'{' | b'}' | b'^' | b'$' => {
                    index += 1;
                }
                _ => {
                    self.mark_content(&mut groups);
                    index += 1;
                }
            }
        }

        if let Some(group) = groups.last()
            && group.seen_pipe
            && !group.current_has_content
        {
            self.has_empty_alternative = true;
        }
    }

    fn mark_content(&self, groups: &mut SmallVec<[GroupState; 8]>) {
        if let Some(group) = groups.last_mut() {
            group.current_has_content = true;
        }
    }
}

fn array_element_expression<'a>(
    element: &'a ArrayExpressionElement<'a>,
) -> Option<&'a Expression<'a>> {
    element.as_expression()
}

fn string_literal_value_with_span<'a>(expression: &'a Expression<'a>) -> Option<(&'a str, Span)> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Some((literal.value.as_str(), literal.span)),
        _ => None,
    }
}

fn duplicate_flag(flags: &str) -> Option<&str> {
    let mut seen = [false; 128];
    for (start, ch) in flags.char_indices() {
        let code = ch as usize;
        if code < seen.len() {
            if seen[code] {
                return Some(&flags[start..start + ch.len_utf8()]);
            }
            seen[code] = true;
        }
    }
    None
}

fn sorted_flags(flags: &str) -> CompactString {
    let mut chars = SmallVec::<[char; 8]>::new();
    chars.extend(flags.chars());
    chars.sort_unstable();
    let mut out = CompactString::new("");
    for ch in chars {
        out.push(ch);
    }
    out
}

fn skip_escape(bytes: &[u8], index: usize) -> usize {
    if index + 1 >= bytes.len() {
        return index + 1;
    }
    match bytes[index + 1] {
        b'u' if index + 2 < bytes.len() && bytes[index + 2] == b'{' => {
            let mut cursor = index + 3;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                cursor += 1;
            }
            cursor.saturating_add(1).min(bytes.len())
        }
        b'u' => (index + 6).min(bytes.len()),
        b'x' => (index + 4).min(bytes.len()),
        _ => (index + 2).min(bytes.len()),
    }
}

fn find_class_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut index = open + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = skip_escape(bytes, index),
            b']' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

fn group_prefix(bytes: &[u8], open: usize) -> (bool, bool, usize) {
    if bytes.get(open + 1) != Some(&b'?') {
        return (true, true, open + 1);
    }
    match bytes.get(open + 2).copied() {
        Some(b':') => (true, false, open + 3),
        Some(b'=') | Some(b'!') => (false, false, open + 3),
        Some(b'<') => {
            if matches!(bytes.get(open + 3), Some(b'=') | Some(b'!')) {
                (false, false, open + 4)
            } else {
                let mut cursor = open + 3;
                while cursor < bytes.len() && bytes[cursor] != b'>' {
                    cursor += 1;
                }
                (true, true, cursor.saturating_add(1).min(bytes.len()))
            }
        }
        _ => (false, false, open + 2),
    }
}

fn is_zero_quantifier(bytes: &[u8], open: usize) -> bool {
    let mut cursor = open + 1;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    if cursor == open + 1 {
        return false;
    }
    let first = std::str::from_utf8(&bytes[open + 1..cursor]).unwrap_or("");
    if first != "0" {
        return false;
    }
    if bytes.get(cursor) == Some(&b'}') {
        return true;
    }
    if bytes.get(cursor) != Some(&b',') {
        return false;
    }
    cursor += 1;
    let second_start = cursor;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'}') {
        return false;
    }
    if cursor == second_start {
        return false;
    }
    std::str::from_utf8(&bytes[second_start..cursor]).unwrap_or("") == "0"
}

fn first_octal_escape(pattern: &str) -> Option<&str> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index + 2 < bytes.len() {
        if bytes[index] == b'\\'
            && bytes[index + 1] == b'0'
            && matches!(bytes[index + 2], b'0'..=b'7')
        {
            let mut end = index + 3;
            while end < bytes.len() && matches!(bytes[end], b'0'..=b'7') {
                end += 1;
            }
            return Some(&pattern[index..end]);
        }
        index = if bytes[index] == b'\\' {
            skip_escape(bytes, index)
        } else {
            index + 1
        };
    }
    None
}

fn first_control_character(pattern: &str) -> Option<char> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            if let Some(ch) = escaped_control_character(bytes, index) {
                return Some(ch);
            }
            index = skip_escape(bytes, index);
            continue;
        }

        let Some(ch) = pattern[index..].chars().next() else {
            break;
        };
        if ch <= '\u{1f}' {
            return Some(ch);
        }
        index += ch.len_utf8();
    }
    None
}

fn escaped_control_character(bytes: &[u8], index: usize) -> Option<char> {
    let code = match bytes.get(index + 1).copied()? {
        b'x' if index + 3 < bytes.len() => {
            u32::from((hex_value(bytes[index + 2])? << 4) | hex_value(bytes[index + 3])?)
        }
        b'u' if index + 2 < bytes.len() && bytes[index + 2] == b'{' => {
            let mut cursor = index + 3;
            let mut value = 0u32;
            let mut saw_digit = false;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                value = (value << 4) | u32::from(hex_value(bytes[cursor])?);
                saw_digit = true;
                cursor += 1;
            }
            if !saw_digit || bytes.get(cursor) != Some(&b'}') {
                return None;
            }
            value
        }
        b'u' if index + 5 < bytes.len() => {
            let mut value = 0u32;
            for byte in &bytes[index + 2..index + 6] {
                value = (value << 4) | u32::from(hex_value(*byte)?);
            }
            value
        }
        _ => return None,
    };
    if code <= 0x1f {
        char::from_u32(code)
    } else {
        None
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn mention_char(ch: char) -> CompactString {
    let mut text = CompactString::new("U+");
    let code = ch as u32;
    let mut buf = [0u8; 6];
    let mut value = code;
    let mut cursor = buf.len();
    if value == 0 {
        cursor -= 1;
        buf[cursor] = b'0';
    } else {
        while value > 0 {
            cursor -= 1;
            let digit = (value & 0xf) as u8;
            buf[cursor] = if digit < 10 {
                b'0' + digit
            } else {
                b'A' + (digit - 10)
            };
            value >>= 4;
        }
    }
    for _ in 0..(4usize.saturating_sub(buf.len() - cursor)) {
        text.push('0');
    }
    if let Ok(hex) = std::str::from_utf8(&buf[cursor..]) {
        text.push_str(hex);
    }
    text
}

#[cfg(test)]
mod tests {
    use oxlint_plugins_carton::SmallVec;

    use super::{implemented_regexp_rule_names, scan_regexp};

    fn ids(source: &str) -> SmallVec<[(&'static str, &'static str); 8]> {
        scan_regexp(source, "fixture.js")
            .into_iter()
            .map(|diagnostic| (diagnostic.rule_name, diagnostic.message_id))
            .collect()
    }

    #[test]
    fn exposes_initial_regexp_rule_names() {
        assert_eq!(
            implemented_regexp_rule_names(),
            &[
                "no-invalid-regexp",
                "no-empty-character-class",
                "no-empty-group",
                "no-empty-capturing-group",
                "no-empty-alternative",
                "no-zero-quantifier",
                "no-octal",
                "no-control-character",
                "sort-flags",
                "require-unicode-regexp",
            ]
        );
    }

    #[test]
    fn scans_literal_pattern_rules() {
        assert_eq!(
            ids("const a = /[]/u;").as_slice(),
            &[("no-empty-character-class", "empty")]
        );
        assert_eq!(ids("const a = /()/u;").len(), 2);
        assert_eq!(
            ids("const a = /a|/u;").as_slice(),
            &[("no-empty-alternative", "empty")]
        );
        assert_eq!(
            ids("const a = /a{0}/u;").as_slice(),
            &[("no-zero-quantifier", "unexpected")]
        );
    }

    #[test]
    fn scans_constructor_patterns_and_flags() {
        assert_eq!(
            ids("const a = new RegExp('[]', 'u');").as_slice(),
            &[("no-empty-character-class", "empty")]
        );
        assert_eq!(
            ids("const a = new RegExp('[', 'u');").as_slice(),
            &[("no-invalid-regexp", "error")]
        );
        assert_eq!(
            ids("const a = new RegExp('a', 'gg');").as_slice(),
            &[("no-invalid-regexp", "duplicateFlag")]
        );
        assert_eq!(
            ids("const a = RegExp('a', 'vu');").as_slice(),
            &[("no-invalid-regexp", "uvFlag")]
        );
    }

    #[test]
    fn scans_style_and_legacy_rules() {
        assert_eq!(
            ids("const a = /a/mi;").as_slice(),
            &[
                ("sort-flags", "sortFlags"),
                ("require-unicode-regexp", "require"),
            ]
        );
        assert_eq!(
            ids("const a = /\\07/u;").as_slice(),
            &[("no-octal", "unexpected")]
        );
        assert_eq!(
            ids("const a = new RegExp('\\u{1}');").as_slice(),
            &[
                ("require-unicode-regexp", "require"),
                ("no-control-character", "unexpected"),
            ]
        );
    }
}
