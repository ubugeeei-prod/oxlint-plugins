//! Expression-level AST traversal for the security scanner.

use oxc_ast::ast::*;

use crate::helpers::{array_element_expression, is_unsafe_regex};
use crate::scanner::Scanner;
use crate::ParentKind;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_expression(
        &mut self,
        expression: &'a Expression<'a>,
        parent: ParentKind,
    ) {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => {
                self.check_call_expression(call, parent);
                self.scan_expression(&call.callee, ParentKind::CallCallee);
                for argument in &call.arguments {
                    self.scan_argument(argument, ParentKind::CallArgument);
                }
            }
            Expression::NewExpression(new_expression) => {
                self.check_new_expression(new_expression);
                self.scan_expression(&new_expression.callee, ParentKind::NewCallee);
                for argument in &new_expression.arguments {
                    self.scan_argument(argument, ParentKind::NewArgument);
                }
            }
            Expression::AssignmentExpression(assignment) => {
                self.check_disable_mustache_escape(
                    assignment.span,
                    &assignment.left,
                    &assignment.right,
                );
                self.scan_assignment_target(&assignment.left, ParentKind::AssignmentLeft);
                self.scan_expression(&assignment.right, ParentKind::AssignmentRight);
            }
            Expression::StaticMemberExpression(member) => {
                if member.property.name == "pseudoRandomBytes" {
                    self.report("detect-pseudoRandomBytes", "found", member.span);
                }
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            Expression::ComputedMemberExpression(member) => {
                self.check_object_injection(member.span, &member.expression, parent);
                self.scan_expression(&member.object, ParentKind::MemberObject);
                self.scan_expression(&member.expression, ParentKind::Other);
            }
            Expression::BinaryExpression(binary) => {
                self.scan_expression(&binary.left, ParentKind::Other);
                self.scan_expression(&binary.right, ParentKind::Other);
            }
            Expression::LogicalExpression(logical) => {
                self.scan_expression(&logical.left, ParentKind::Other);
                self.scan_expression(&logical.right, ParentKind::Other);
            }
            Expression::ConditionalExpression(conditional) => {
                self.scan_expression(&conditional.test, ParentKind::Other);
                self.scan_expression(&conditional.consequent, ParentKind::Other);
                self.scan_expression(&conditional.alternate, ParentKind::Other);
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(expression) = array_element_expression(element) {
                        self.scan_expression(expression, ParentKind::Other);
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
                            self.scan_expression(&property.value, ParentKind::Other);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(&spread.argument, ParentKind::Other);
                        }
                    }
                }
            }
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.scan_expression(&tagged.tag, ParentKind::Other);
                for expression in &tagged.quasi.expressions {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Expression::FunctionExpression(function) => self.scan_function(function),
            Expression::ArrowFunctionExpression(function) => {
                self.push_scope();
                for param in &function.params.items {
                    self.bind_pattern_unknown(&param.pattern);
                }
                for statement in &function.body.statements {
                    self.scan_statement(statement);
                }
                self.pop_scope();
            }
            Expression::ClassExpression(class) => self.scan_class(class),
            Expression::SequenceExpression(sequence) => {
                for expression in &sequence.expressions {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Expression::AwaitExpression(await_expression) => {
                self.scan_expression(&await_expression.argument, ParentKind::Other);
            }
            Expression::UnaryExpression(unary) => {
                self.scan_expression(&unary.argument, ParentKind::Other);
            }
            Expression::UpdateExpression(_) => {}
            Expression::YieldExpression(yield_expression) => {
                if let Some(argument) = &yield_expression.argument {
                    self.scan_expression(argument, ParentKind::Other);
                }
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    self.check_call_expression(call, parent);
                    self.scan_expression(&call.callee, ParentKind::CallCallee);
                    for argument in &call.arguments {
                        self.scan_argument(argument, ParentKind::CallArgument);
                    }
                }
                ChainElement::StaticMemberExpression(member) => {
                    self.scan_expression(&member.object, ParentKind::MemberObject);
                }
                ChainElement::ComputedMemberExpression(member) => {
                    self.check_object_injection(member.span, &member.expression, parent);
                    self.scan_expression(&member.object, ParentKind::MemberObject);
                    self.scan_expression(&member.expression, ParentKind::Other);
                }
                ChainElement::PrivateFieldExpression(member) => {
                    self.scan_expression(&member.object, ParentKind::MemberObject);
                }
                ChainElement::TSNonNullExpression(expression) => {
                    self.scan_expression(&expression.expression, parent);
                }
            },
            Expression::RegExpLiteral(literal)
                if is_unsafe_regex(literal.regex.pattern.text.as_str()) =>
            {
                self.report("detect-unsafe-regex", "literal", literal.span);
            }
            _ => {}
        }
    }

    pub(crate) fn scan_property_key(&mut self, key: &'a PropertyKey<'a>) {
        if let Some(expression) = key.as_expression() {
            self.scan_expression(expression, ParentKind::Other);
        }
    }

    pub(crate) fn scan_argument(&mut self, argument: &'a Argument<'a>, parent: ParentKind) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression, parent);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument, parent);
        }
    }

    pub(crate) fn scan_assignment_target(
        &mut self,
        target: &'a AssignmentTarget<'a>,
        parent: ParentKind,
    ) {
        match target {
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.check_object_injection(member.span, &member.expression, parent);
                self.scan_expression(&member.object, ParentKind::MemberObject);
                self.scan_expression(&member.expression, ParentKind::Other);
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            AssignmentTarget::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            AssignmentTarget::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            AssignmentTarget::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            AssignmentTarget::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            AssignmentTarget::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            _ => {}
        }
    }
}
