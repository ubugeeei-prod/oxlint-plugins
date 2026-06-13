//! AST scanner for the react-hooks port. Contains the `Scanner` struct and
//! every traversal / rule check method as an `impl Scanner` block.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::*;
use crate::{
    Diagnostic, DiagnosticData, FunctionFrame, HookCall, LineIndex, is_hook_name,
    is_react_component_name,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    pub(crate) frames: SmallVec<[FunctionFrame; 8]>,
    pub(crate) class_depth: u32,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan_statement_list(&mut self, statements: &'a [Statement<'a>]) {
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
    pub(crate) fn is_inside_valid_scope(&self) -> bool {
        self.valid_scope || self.inside_valid_scope
    }
}
