//! Constant-expression detection, copy-pattern recognition, and spread-leak
//! analysis used across e18e rules.

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, Expression, ObjectPropertyKind, Statement,
};

use crate::helpers::ast::{expression_body, static_member_callee};

pub(crate) fn constant_callback_value<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a Expression<'a>> {
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

pub(crate) fn is_constant_expression(expression: &Expression<'_>) -> bool {
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

pub(crate) fn copy_pattern_source<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a Expression<'a>> {
    match expression.get_inner_expression() {
        Expression::ArrayExpression(array) => {
            crate::helpers::single_spread_element(array).map(|spread| &spread.argument)
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

pub(crate) fn copy_pattern_optional(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::StaticMemberExpression(member) => member.optional,
        Expression::CallExpression(call) => call.optional,
        _ => false,
    }
}

pub(crate) fn function_body_contains_spread(expression: &Expression<'_>) -> bool {
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

pub(crate) fn statement_contains_spread(statement: &Statement<'_>) -> bool {
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

pub(crate) fn expression_contains_spread(expression: &Expression<'_>) -> bool {
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
