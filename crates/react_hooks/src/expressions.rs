//! Expression-level AST traversal for the react-hooks scanner.

use oxc_ast::ast::*;

use crate::helpers::{
    assignment_target_name, function_name, is_component_callback_callee,
};
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_expression(&mut self, expression: &'a Expression<'a>) {
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

    pub(crate) fn scan_call_expression(&mut self, call: &'a CallExpression<'a>) {
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

    pub(crate) fn scan_argument(
        &mut self,
        argument: &'a Argument<'a>,
        special_component_callback: bool,
    ) {
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

    pub(crate) fn scan_array_element(&mut self, element: &'a ArrayExpressionElement<'a>) {
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

    pub(crate) fn scan_property_key(&mut self, key: &'a PropertyKey<'a>) {
        if let Some(expression) = key.as_expression() {
            self.scan_expression(expression);
        }
    }

    pub(crate) fn scan_assignment_target(&mut self, target: &'a AssignmentTarget<'a>) {
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

    pub(crate) fn scan_chain_element(&mut self, element: &'a ChainElement<'a>) {
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
}
