//! Expression-level AST traversal for the functional scanner.

use oxc_ast::ast::*;

use crate::FunctionContext;
use crate::FunctionParamMeta;
use crate::helpers::{
    assignment_target_is_member, is_identifier_expression, is_mutating_call, is_static_call,
};
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_expression(
        &mut self,
        expression: &'a Expression<'a>,
        context: FunctionContext,
    ) {
        match expression.get_inner_expression() {
            Expression::Identifier(identifier) => {
                let is_arguments = identifier.name == "arguments";
                let allow_args = self.options.allow_arguments_keyword;
                if is_arguments && !allow_args {
                    self.report(
                        "functional-parameters",
                        "arguments",
                        "Unexpected use of `arguments`. Use regular function arguments instead.",
                        identifier.span,
                    );
                }
            }
            Expression::ThisExpression(expression) => {
                self.report(
                    "no-this-expressions",
                    "generic",
                    "Unexpected this, use functions not classes.",
                    expression.span,
                );
            }
            Expression::CallExpression(call) => self.scan_call_expression(call, context),
            Expression::NewExpression(expression) => self.scan_new_expression(expression, context),
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
                self.scan_computed_member_expression(member, context);
            }
            Expression::AssignmentExpression(expression) => {
                self.scan_assignment_expression(expression, context);
            }
            Expression::UpdateExpression(expression) => {
                self.report(
                    "immutable-data",
                    "generic",
                    "Modifying an existing object/array is not allowed.",
                    expression.span,
                );
            }
            Expression::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function, FunctionParamMeta::default());
            }
            Expression::FunctionExpression(function) => {
                let fn_name: Option<&'a str> =
                    function.id.as_ref().map(|id| id.name.as_str());
                let meta = FunctionParamMeta {
                    name: fn_name,
                    ..FunctionParamMeta::default()
                };
                self.scan_function(function, meta);
            }
            Expression::ClassExpression(class) => self.scan_class(class, context),
            Expression::ObjectExpression(expression) => {
                for property in &expression.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed {
                                self.scan_property_key(&property.key, context);
                            }
                            // For function/arrow values in an object literal, thread the
                            // property name and getter/setter flag into the meta so that
                            // `ignoreIdentifierPattern` and `ignoreGettersAndSetters` work.
                            let prop_name: Option<&'a str> =
                                if let PropertyKey::StaticIdentifier(id) = &property.key {
                                    Some(id.name.as_str())
                                } else if let PropertyKey::StringLiteral(lit) = &property.key {
                                    Some(lit.value.as_str())
                                } else {
                                    None
                                };
                            let is_getter_setter = matches!(
                                property.kind,
                                PropertyKind::Get | PropertyKind::Set
                            );
                            match property.value.get_inner_expression() {
                                Expression::FunctionExpression(func) => {
                                    let name_from_id: Option<&'a str> =
                                        func.id.as_ref().map(|id| id.name.as_str());
                                    let effective_name = name_from_id.or(prop_name);
                                    let meta = FunctionParamMeta {
                                        name: effective_name,
                                        is_getter_setter,
                                        ..FunctionParamMeta::default()
                                    };
                                    self.scan_function(func, meta);
                                }
                                Expression::ArrowFunctionExpression(func) => {
                                    let meta = FunctionParamMeta {
                                        name: prop_name,
                                        is_getter_setter,
                                        ..FunctionParamMeta::default()
                                    };
                                    self.scan_arrow_function(func, meta);
                                }
                                _ => {
                                    self.scan_expression(&property.value, context);
                                }
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(&spread.argument, context);
                        }
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
                self.scan_expression(&expression.test, context);
                self.scan_expression(&expression.consequent, context);
                self.scan_expression(&expression.alternate, context);
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
            Expression::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            Expression::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, context)
            }
            _ => {}
        }
    }

    fn scan_array_element(
        &mut self,
        element: &'a ArrayExpressionElement<'a>,
        context: FunctionContext,
    ) {
        match element {
            ArrayExpressionElement::SpreadElement(spread) => {
                self.scan_expression(&spread.argument, context)
            }
            ArrayExpressionElement::CallExpression(call) => {
                self.scan_call_expression(call, context)
            }
            ArrayExpressionElement::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            ArrayExpressionElement::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            ArrayExpressionElement::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function, FunctionParamMeta::default());
            }
            ArrayExpressionElement::FunctionExpression(function) => {
                self.scan_function(function, FunctionParamMeta::default());
            }
            ArrayExpressionElement::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, context);
                }
            }
            ArrayExpressionElement::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property {
                        self.scan_expression(&property.value, context);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn scan_argument(&mut self, argument: &'a Argument<'a>, context: FunctionContext) {
        match argument {
            Argument::SpreadElement(spread) => self.scan_expression(&spread.argument, context),
            Argument::Identifier(identifier) => {
                let is_arguments = identifier.name == "arguments";
                let allow_args = self.options.allow_arguments_keyword;
                if is_arguments && !allow_args {
                    self.report(
                        "functional-parameters",
                        "arguments",
                        "Unexpected use of `arguments`. Use regular function arguments instead.",
                        identifier.span,
                    );
                }
            }
            Argument::CallExpression(call) => self.scan_call_expression(call, context),
            Argument::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            Argument::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            Argument::ArrowFunctionExpression(function) => {
                self.scan_arrow_function(function, FunctionParamMeta::default());
            }
            Argument::FunctionExpression(function) => {
                self.scan_function(function, FunctionParamMeta::default());
            }
            Argument::ClassExpression(class) => self.scan_class(class, context),
            Argument::ArrayExpression(expression) => {
                for element in &expression.elements {
                    self.scan_array_element(element, context);
                }
            }
            Argument::ObjectExpression(expression) => {
                for property in &expression.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property {
                        self.scan_expression(&property.value, context);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn scan_property_key(&mut self, key: &'a PropertyKey<'a>, context: FunctionContext) {
        match key {
            PropertyKey::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            PropertyKey::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            PropertyKey::CallExpression(call) => self.scan_call_expression(call, context),
            _ => {}
        }
    }

    pub(crate) fn scan_assignment_expression(
        &mut self,
        expression: &'a oxc_ast::ast::AssignmentExpression<'a>,
        context: FunctionContext,
    ) {
        if assignment_target_is_member(&expression.left) {
            self.report(
                "immutable-data",
                "generic",
                "Modifying an existing object/array is not allowed.",
                expression.span,
            );
        }
        self.scan_assignment_target(&expression.left, context);
        self.scan_expression(&expression.right, context);
    }

    fn scan_assignment_target(
        &mut self,
        target: &'a AssignmentTarget<'a>,
        context: FunctionContext,
    ) {
        match target {
            AssignmentTarget::StaticMemberExpression(member) => {
                self.scan_static_member_expression(member, context);
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.scan_computed_member_expression(member, context);
            }
            AssignmentTarget::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object, context);
            }
            AssignmentTarget::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            AssignmentTarget::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            AssignmentTarget::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            AssignmentTarget::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, context);
            }
            _ => {}
        }
    }

    pub(crate) fn scan_static_member_expression(
        &mut self,
        member: &'a StaticMemberExpression<'a>,
        context: FunctionContext,
    ) {
        self.scan_expression(&member.object, context);
    }

    pub(crate) fn scan_computed_member_expression(
        &mut self,
        member: &'a ComputedMemberExpression<'a>,
        context: FunctionContext,
    ) {
        self.scan_expression(&member.object, context);
        self.scan_expression(&member.expression, context);
    }

    pub(crate) fn scan_call_expression(
        &mut self,
        call: &'a CallExpression<'a>,
        context: FunctionContext,
    ) {
        if is_static_call(call, "Promise", "reject") {
            self.report(
                "no-promise-reject",
                "generic",
                "Unexpected rejection, resolve an error instead.",
                call.span,
            );
        }
        if is_mutating_call(call) {
            self.report(
                "immutable-data",
                "generic",
                "Modifying an existing object/array is not allowed.",
                call.span,
            );
        }

        // Determine if the callee itself is a function (IIFE).
        let callee_inner = call.callee.get_inner_expression();
        let callee_is_iife_fn = matches!(
            callee_inner,
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
        );
        if callee_is_iife_fn {
            match callee_inner {
                Expression::FunctionExpression(func) => {
                    let fn_name: Option<&'a str> =
                        func.id.as_ref().map(|id| id.name.as_str());
                    let meta = FunctionParamMeta {
                        name: fn_name,
                        is_iife: true,
                        ..FunctionParamMeta::default()
                    };
                    self.scan_function(func, meta);
                }
                Expression::ArrowFunctionExpression(func) => {
                    let meta = FunctionParamMeta {
                        is_iife: true,
                        ..FunctionParamMeta::default()
                    };
                    self.scan_arrow_function(func, meta);
                }
                _ => {}
            }
        } else {
            self.scan_expression(&call.callee, context);
        }

        // Determine the callee property name for ignorePrefixSelector.
        let callee_prop_name: Option<&'a str> =
            if let Expression::StaticMemberExpression(member) = callee_inner {
                Some(member.property.name.as_str())
            } else {
                None
            };

        for argument in &call.arguments {
            // For function/arrow arguments, scan them as lambda args with the
            // enclosing call property set (for ignorePrefixSelector). This avoids
            // double-scanning via the generic scan_argument path.
            match argument {
                Argument::FunctionExpression(func) => {
                    let fn_name: Option<&'a str> =
                        func.id.as_ref().map(|id| id.name.as_str());
                    let meta = FunctionParamMeta {
                        name: fn_name,
                        is_lambda_arg: true,
                        enclosing_call_property: callee_prop_name,
                        ..FunctionParamMeta::default()
                    };
                    self.scan_function(func, meta);
                }
                Argument::ArrowFunctionExpression(func) => {
                    let meta = FunctionParamMeta {
                        is_lambda_arg: true,
                        enclosing_call_property: callee_prop_name,
                        ..FunctionParamMeta::default()
                    };
                    self.scan_arrow_function(func, meta);
                }
                _ => {
                    self.scan_argument(argument, context);
                }
            }
        }
    }

    pub(crate) fn scan_new_expression(
        &mut self,
        expression: &'a NewExpression<'a>,
        context: FunctionContext,
    ) {
        // `new Promise(executor)` can reject only when the executor declares a
        // second `reject` parameter, mirroring upstream's `arguments[0].params
        // .length >= 2` check.
        let executor_can_reject = expression.arguments.first().is_some_and(|argument| {
            let params = match argument {
                Argument::ArrowFunctionExpression(function) => Some(&function.params),
                Argument::FunctionExpression(function) => Some(&function.params),
                _ => None,
            };
            params.is_some_and(|params| params.items.len() >= 2)
        });
        if is_identifier_expression(&expression.callee, "Promise") && executor_can_reject {
            self.report(
                "no-promise-reject",
                "generic",
                "Unexpected rejection, resolve an error instead.",
                expression.span,
            );
        }
        self.scan_expression(&expression.callee, context);
        for argument in &expression.arguments {
            self.scan_argument(argument, context);
        }
    }
}
