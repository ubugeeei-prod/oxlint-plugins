#![doc = "Rust implementation of eslint-plugin-react-hooks rule logic."]

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionExpression, AssignmentTarget, BindingPattern,
    CallExpression, ChainElement, Class, ClassElement, Declaration, ExportDefaultDeclarationKind,
    Expression, ForStatementInit, ForStatementLeft, Function, FunctionBody, ObjectPropertyKind,
    PropertyKey, Statement, VariableDeclaration,
};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub const RULE_NAMES: [&str; 1] = ["rules-of-hooks"];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub hook: Option<CompactString>,
    pub function_name: Option<CompactString>,
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

#[derive(Clone, Debug)]
struct FunctionFrame {
    name: Option<CompactString>,
    valid_scope: bool,
    inside_valid_scope: bool,
    async_function: bool,
    in_class: bool,
    conditional_depth: u32,
    loop_depth: u32,
    try_depth: u32,
    possible_early_return: bool,
}

#[derive(Clone, Debug)]
struct HookCall {
    name: CompactString,
    span: Span,
    is_use: bool,
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

pub fn implemented_react_hooks_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn is_react_component_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
}

pub fn is_hook_name(name: &str) -> bool {
    if name == "use" {
        return true;
    }

    let Some(rest) = name.strip_prefix("use") else {
        return false;
    };
    let Some(next) = rest.chars().next() else {
        return false;
    };
    next.is_ascii_uppercase() || next.is_ascii_digit()
}

pub fn scan_react_hooks(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = source_type_for_filename(filename);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        frames: SmallVec::new(),
        class_depth: 0,
    };
    scanner.scan_statement_list(&parser_return.program.body);
    scanner.diagnostics
}

fn source_type_for_filename(filename: &str) -> SourceType {
    if filename.ends_with(".tsx") {
        SourceType::tsx()
    } else if filename.ends_with(".jsx") {
        SourceType::jsx()
    } else {
        SourceType::from_path(Path::new(filename))
            .unwrap_or_else(|_| SourceType::mjs())
            .with_module(true)
    }
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 16]>,
    frames: SmallVec<[FunctionFrame; 8]>,
    class_depth: u32,
}

impl<'a> Scanner<'a> {
    fn scan_statement_list(&mut self, statements: &'a [Statement<'a>]) {
        for statement in statements {
            self.scan_statement(statement);
        }
    }

    fn scan_statement(&mut self, statement: &'a Statement<'a>) {
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

    fn scan_declaration(&mut self, declaration: &'a Declaration<'a>) {
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

    fn scan_expression(&mut self, expression: &'a Expression<'a>) {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => {
                self.scan_call_expression(call);
            }
            Expression::NewExpression(new_expression) => {
                self.scan_expression(&new_expression.callee);
                for argument in &new_expression.arguments {
                    self.scan_argument(argument, false);
                }
            }
            Expression::AssignmentExpression(assignment) => {
                self.scan_assignment_target(&assignment.left);
                let name = assignment_target_name(&assignment.left);
                if let Expression::ArrowFunctionExpression(function) =
                    assignment.right.get_inner_expression()
                {
                    self.scan_arrow_function(function, name, false);
                } else if let Expression::FunctionExpression(function) =
                    assignment.right.get_inner_expression()
                {
                    self.scan_function(function, name.or_else(|| function_name(function)), false);
                } else {
                    self.scan_expression(&assignment.right);
                }
            }
            Expression::StaticMemberExpression(member) => {
                self.scan_expression(&member.object);
            }
            Expression::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object);
                self.scan_expression(&member.expression);
            }
            Expression::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object);
            }
            Expression::BinaryExpression(binary) => {
                self.scan_expression(&binary.left);
                self.scan_expression(&binary.right);
            }
            Expression::LogicalExpression(logical) => {
                self.scan_expression(&logical.left);
                self.with_conditional(|scanner| scanner.scan_expression(&logical.right));
            }
            Expression::ConditionalExpression(conditional) => {
                self.scan_expression(&conditional.test);
                self.with_conditional(|scanner| {
                    scanner.scan_expression(&conditional.consequent);
                    scanner.scan_expression(&conditional.alternate);
                });
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
            Expression::FunctionExpression(function) => {
                self.scan_function(function, function_name(function), false);
            }
            Expression::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function, None, false);
            }
            Expression::ClassExpression(class) => {
                self.scan_class(class);
            }
            Expression::SequenceExpression(sequence) => {
                for expression in &sequence.expressions {
                    self.scan_expression(expression);
                }
            }
            Expression::AwaitExpression(await_expression) => {
                self.scan_expression(&await_expression.argument);
            }
            Expression::UnaryExpression(unary) => {
                self.scan_expression(&unary.argument);
            }
            Expression::UpdateExpression(_) => {}
            Expression::YieldExpression(yield_expression) => {
                if let Some(argument) = &yield_expression.argument {
                    self.scan_expression(argument);
                }
            }
            Expression::ChainExpression(chain) => {
                self.scan_chain_element(&chain.expression);
            }
            _ => {}
        }
    }

    fn scan_call_expression(&mut self, call: &'a CallExpression<'a>) {
        if let Some(hook_call) = self.hook_call(call) {
            self.report_hook_call(&hook_call);
        }

        self.scan_expression(&call.callee);
        for (index, argument) in call.arguments.iter().enumerate() {
            self.scan_argument(
                argument,
                index == 0 && is_component_callback_callee(&call.callee),
            );
        }
    }

    fn scan_argument(&mut self, argument: &'a Argument<'a>, special_component_callback: bool) {
        match argument {
            Argument::FunctionExpression(function) => {
                self.scan_function(
                    function,
                    function_name(function),
                    special_component_callback,
                );
            }
            Argument::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function, None, special_component_callback);
            }
            Argument::SpreadElement(spread) => {
                self.scan_expression(&spread.argument);
            }
            _ => {
                if let Some(expression) = argument.as_expression() {
                    self.scan_expression(expression);
                }
            }
        }
    }

    fn scan_array_element(&mut self, element: &'a ArrayExpressionElement<'a>) {
        match element {
            ArrayExpressionElement::SpreadElement(spread) => {
                self.scan_expression(&spread.argument);
            }
            _ => {
                if let Some(expression) = element.as_expression() {
                    self.scan_expression(expression);
                }
            }
        }
    }

    fn scan_property_key(&mut self, key: &'a PropertyKey<'a>) {
        if let Some(expression) = key.as_expression() {
            self.scan_expression(expression);
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
                self.scan_expression(&expression.expression);
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

    fn scan_chain_element(&mut self, element: &'a ChainElement<'a>) {
        match element {
            ChainElement::CallExpression(call) => {
                self.scan_call_expression(call);
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
        }
    }

    fn scan_function(
        &mut self,
        function: &'a Function<'a>,
        name: Option<&'a str>,
        special_component_callback: bool,
    ) {
        let Some(body) = function.body.as_deref() else {
            return;
        };
        self.enter_function(
            name,
            function.r#async,
            self.class_depth > 0,
            special_component_callback,
            body,
        );
    }

    fn scan_arrow_function(
        &mut self,
        function: &'a ArrowFunctionExpression<'a>,
        name: Option<&'a str>,
        special_component_callback: bool,
    ) {
        self.enter_function(
            name,
            function.r#async,
            self.class_depth > 0,
            special_component_callback,
            &function.body,
        );
    }

    fn enter_function(
        &mut self,
        name: Option<&'a str>,
        async_function: bool,
        in_class: bool,
        special_component_callback: bool,
        body: &'a FunctionBody<'a>,
    ) {
        let valid_scope = special_component_callback
            || name.is_some_and(|name| is_react_component_name(name) || is_hook_name(name));
        let inside_valid_scope = valid_scope
            || self
                .current_frame()
                .is_some_and(FunctionFrame::is_inside_valid_scope);
        self.frames.push(FunctionFrame {
            name: name.map(CompactString::from),
            valid_scope,
            inside_valid_scope,
            async_function,
            in_class,
            conditional_depth: 0,
            loop_depth: 0,
            try_depth: 0,
            possible_early_return: false,
        });
        self.scan_statement_list(&body.statements);
        let _ = self.frames.pop();
    }

    fn scan_class(&mut self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class);
        }

        self.class_depth += 1;
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    self.scan_statement_list(&block.body);
                }
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value, method_name(&method.key), false);
                }
                ClassElement::PropertyDefinition(property) => {
                    if property.computed {
                        self.scan_property_key(&property.key);
                    }
                    if let Some(value) = &property.value {
                        self.scan_expression(value);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if property.computed {
                        self.scan_property_key(&property.key);
                    }
                    if let Some(value) = &property.value {
                        self.scan_expression(value);
                    }
                }
                ClassElement::TSIndexSignature(_) => {}
            }
        }
        self.class_depth = self.class_depth.saturating_sub(1);
    }

    fn hook_call(&self, call: &'a CallExpression<'a>) -> Option<HookCall> {
        match call.callee.get_inner_expression() {
            Expression::Identifier(identifier) if is_hook_name(identifier.name.as_str()) => {
                Some(HookCall {
                    name: CompactString::from(identifier.name.as_str()),
                    span: identifier.span,
                    is_use: identifier.name == "use",
                })
            }
            Expression::StaticMemberExpression(member)
                if is_hook_name(member.property.name.as_str())
                    && object_is_pascal_case_identifier(&member.object) =>
            {
                Some(HookCall {
                    name: self.compact_text(member.span),
                    span: member.span,
                    is_use: member.property.name == "use",
                })
            }
            _ => None,
        }
    }

    fn report_hook_call(&mut self, hook_call: &HookCall) {
        let Some(frame) = self.current_frame() else {
            if self.class_depth > 0 {
                self.report("class", hook_call, None);
            } else {
                self.report("topLevel", hook_call, None);
            }
            return;
        };

        let message_id = if hook_call.is_use && frame.try_depth > 0 {
            Some("tryCatch")
        } else if !hook_call.is_use && frame.loop_depth > 0 {
            Some("loop")
        } else if frame.in_class {
            Some("class")
        } else if !frame.valid_scope {
            if frame.is_inside_valid_scope() {
                Some("callback")
            } else if frame.name.is_some() {
                Some("invalidFunction")
            } else {
                Some("callback")
            }
        } else if !hook_call.is_use && frame.async_function {
            Some("async")
        } else if !hook_call.is_use && (frame.conditional_depth > 0 || frame.possible_early_return)
        {
            Some("conditional")
        } else {
            None
        };

        if let Some(message_id) = message_id {
            let function_name = frame.name.as_ref().cloned();
            self.report(message_id, hook_call, function_name);
        }
    }

    fn report(
        &mut self,
        message_id: &'static str,
        hook_call: &HookCall,
        function_name: Option<CompactString>,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name: "rules-of-hooks",
            message_id,
            data: DiagnosticData {
                hook: Some(hook_call.name.clone()),
                function_name,
            },
            loc: self
                .line_index
                .loc_for_span(self.source_text, hook_call.span),
        });
    }

    fn compact_text(&self, span: Span) -> CompactString {
        let start = span.start as usize;
        let end = span.end as usize;
        self.source_text
            .get(start..end)
            .map_or_else(|| CompactString::from("React Hook"), CompactString::from)
    }

    fn with_conditional(&mut self, f: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.current_frame_mut() {
            frame.conditional_depth += 1;
        }
        f(self);
        if let Some(frame) = self.current_frame_mut() {
            frame.conditional_depth = frame.conditional_depth.saturating_sub(1);
        }
    }

    fn with_loop(&mut self, f: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.current_frame_mut() {
            frame.loop_depth += 1;
        }
        f(self);
        if let Some(frame) = self.current_frame_mut() {
            frame.loop_depth = frame.loop_depth.saturating_sub(1);
        }
    }

    fn with_try(&mut self, f: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.current_frame_mut() {
            frame.try_depth += 1;
        }
        f(self);
        if let Some(frame) = self.current_frame_mut() {
            frame.try_depth = frame.try_depth.saturating_sub(1);
        }
    }

    fn current_frame(&self) -> Option<&FunctionFrame> {
        self.frames.last()
    }

    fn current_frame_mut(&mut self) -> Option<&mut FunctionFrame> {
        self.frames.last_mut()
    }
}

impl FunctionFrame {
    fn is_inside_valid_scope(&self) -> bool {
        self.valid_scope || self.inside_valid_scope
    }
}

fn binding_pattern_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn function_name<'a>(function: &'a Function<'a>) -> Option<&'a str> {
    function
        .id
        .as_ref()
        .map(|identifier| identifier.name.as_str())
}

fn method_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::PrivateIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn assignment_target_name<'a>(target: &'a AssignmentTarget<'a>) -> Option<&'a str> {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(identifier) => Some(identifier.name.as_str()),
        AssignmentTarget::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

fn object_is_pascal_case_identifier(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::Identifier(identifier) if is_react_component_name(identifier.name.as_str())
    )
}

fn is_component_callback_callee(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "forwardRef" | "memo")
        }
        Expression::StaticMemberExpression(member) => {
            matches!(
                member.object.get_inner_expression(),
                Expression::Identifier(identifier) if identifier.name == "React"
            ) && matches!(member.property.name.as_str(), "forwardRef" | "memo")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_hook_name, is_react_component_name, scan_react_hooks};

    fn message_ids(source_text: &str) -> oxlint_plugins_carton::SmallVec<[&'static str; 16]> {
        scan_react_hooks(source_text, "Component.tsx")
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    #[test]
    fn classifies_component_and_hook_names() {
        let cases = [
            ("Component", true, false),
            ("CMS", true, false),
            ("useState", false, true),
            ("use2", false, true),
            ("use", false, true),
            ("use_state", false, false),
            ("component", false, false),
        ];

        for (name, component, hook) in cases {
            assert_eq!(is_react_component_name(name), component);
            assert_eq!(is_hook_name(name), hook);
        }
    }

    #[test]
    fn scans_rules_of_hooks_categories() {
        let cases = [
            ("useState();\n", &["topLevel"][..]),
            (
                "function normal() { useState(); }\n",
                &["invalidFunction"][..],
            ),
            (
                "function Component() { items.map(() => { useState(); }); }\n",
                &["callback"][..],
            ),
            (
                "function Component() { if (cond) { useState(); } }\n",
                &["conditional"][..],
            ),
            (
                "function Component() { if (cond) return null; useState(); }\n",
                &["conditional"][..],
            ),
            (
                "function Component() { while (cond) { useState(); } }\n",
                &["loop"][..],
            ),
            (
                "async function Component() { useState(); }\n",
                &["async"][..],
            ),
            (
                "class App extends React.Component { render() { useState(); } }\n",
                &["class"][..],
            ),
            (
                "function Component() { try { use(resource); } catch (error) {} }\n",
                &["tryCatch"][..],
            ),
            (
                "function Component() { if (cond) { use(resource); } }\n",
                &[][..],
            ),
        ];

        for (source_text, expected) in cases {
            assert_eq!(message_ids(source_text).as_slice(), expected);
        }
    }
}
