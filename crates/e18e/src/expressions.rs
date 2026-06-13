//! Expression-level AST traversal and per-node check dispatchers for the
//! e18e scanner.

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, AssignmentTarget, CallExpression, ChainElement, Expression,
    NewExpression, ObjectPropertyKind, RegExpFlags,
};

use crate::helpers::{ExprContext, expression_body};
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_expression(&mut self, expression: &'a Expression<'a>, context: ExprContext) {
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

    pub(crate) fn scan_argument(&mut self, argument: &'a Argument<'a>) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression, ExprContext::Other);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument, ExprContext::Other);
        }
    }

    pub(crate) fn scan_array_element(&mut self, element: &'a ArrayExpressionElement<'a>) {
        if let Some(expression) = element.as_expression() {
            self.scan_expression(expression, ExprContext::Other);
        } else if let ArrayExpressionElement::SpreadElement(spread) = element {
            self.scan_expression(&spread.argument, ExprContext::Other);
        }
    }

    pub(crate) fn scan_assignment_target(&mut self, target: &'a AssignmentTarget<'a>) {
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

    pub(crate) fn check_call_expression(
        &mut self,
        call: &'a CallExpression<'a>,
        context: ExprContext,
    ) {
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

    pub(crate) fn check_new_expression(&mut self, new_expression: &'a NewExpression<'a>) {
        self.check_prefer_static_regex_new(new_expression);
        self.check_prefer_date_now_new(new_expression);
    }

    pub(crate) fn check_static_member_expression(
        &mut self,
        member: &'a oxc_ast::ast::StaticMemberExpression<'a>,
        context: ExprContext,
    ) {
        self.check_filter_length_member(member, context);
    }

    pub(crate) fn check_computed_member_expression(
        &mut self,
        member: &'a oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        self.check_prefer_array_at(member);
    }

    pub(crate) fn check_binary_expression(
        &mut self,
        binary: &'a oxc_ast::ast::BinaryExpression<'a>,
    ) {
        self.check_prefer_includes_binary(binary);
        self.check_no_indexof_equality(binary);
        self.check_prefer_array_some_binary(binary);
    }

    pub(crate) fn check_unary_expression(
        &mut self,
        unary: &'a oxc_ast::ast::UnaryExpression<'a>,
        context: ExprContext,
    ) {
        self.check_prefer_includes_unary(unary);
        self.check_prefer_array_some_unary(unary);
        self.check_prefer_date_now_unary(unary);
        self.check_no_delete_property(unary, context);
    }
}
