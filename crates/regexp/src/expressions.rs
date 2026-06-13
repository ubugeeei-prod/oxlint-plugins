//! Expression-level traversal for the regexp scanner. Split from the
//! statement-level walker in `traversal.rs` to keep each file focused.

use oxc_ast::ast::{Argument, AssignmentTarget, ChainElement, Expression, ObjectPropertyKind, PropertyKey};

use crate::helpers::array_element_expression;
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_expression(&mut self, expression: &'a Expression<'a>) {
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

    pub(crate) fn scan_property_key(&mut self, key: &'a PropertyKey<'a>) {
        if let Some(expression) = key.as_expression() {
            self.scan_expression(expression);
        }
    }

    pub(crate) fn scan_argument(&mut self, argument: &'a Argument<'a>) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument);
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
}
