//! Expression-level AST traversal for the mocha scanner.

use oxc_ast::ast::*;

use crate::ContextKind;
use crate::helpers::is_suite_config_call;
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_expression(&mut self, expression: &'a Expression<'a>, context: ContextKind) {
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

    pub(crate) fn scan_array_element(
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

    pub(crate) fn scan_argument(&mut self, argument: &'a Argument<'a>, context: ContextKind) {
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

    pub(crate) fn scan_property_key(&mut self, key: &'a PropertyKey<'a>, context: ContextKind) {
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

    pub(crate) fn scan_conditional_expression(
        &mut self,
        expression: &'a ConditionalExpression<'a>,
        context: ContextKind,
    ) {
        self.scan_expression(&expression.test, context);
        self.scan_expression(&expression.consequent, context);
        self.scan_expression(&expression.alternate, context);
    }

    pub(crate) fn scan_static_member_expression(
        &mut self,
        member: &'a StaticMemberExpression<'a>,
        context: ContextKind,
    ) {
        self.scan_expression(&member.object, context);
    }

    pub(crate) fn scan_call_expression(
        &mut self,
        call: &'a CallExpression<'a>,
        context: ContextKind,
    ) {
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
}
