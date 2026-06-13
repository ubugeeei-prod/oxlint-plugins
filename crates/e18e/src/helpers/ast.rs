//! AST-shape navigation helpers: callee inspection, structural matches.

use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, CallExpression, Expression, FunctionBody,
    NewExpression, ObjectPropertyKind, PropertyKey, Statement,
};

pub(crate) fn static_member_callee<'a>(
    call: &'a CallExpression<'a>,
) -> Option<(&'a Expression<'a>, &'a str)> {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return None;
    };
    Some((&member.object, member.property.name.as_str()))
}

pub(crate) fn is_static_call(
    call: &CallExpression<'_>,
    object_name: &str,
    property_name: &str,
) -> bool {
    let Some((object, property)) = static_member_callee(call) else {
        return false;
    };
    if !property_name.is_empty() && property != property_name {
        return false;
    }
    matches!(object.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == object_name)
}

pub(crate) fn is_method_call(call: &CallExpression<'_>, method_name: &str) -> bool {
    static_member_callee(call).is_some_and(|(_, property)| property == method_name)
}

pub(crate) fn callee_path(expression: &Expression<'_>) -> Option<String> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.to_string()),
        Expression::StaticMemberExpression(member) => {
            let mut path = callee_path(&member.object)?;
            path.push('.');
            path.push_str(member.property.name.as_str());
            Some(path)
        }
        _ => None,
    }
}

pub(crate) fn single_spread_element<'a>(
    array: &'a ArrayExpression<'a>,
) -> Option<&'a oxc_ast::ast::SpreadElement<'a>> {
    if array.elements.len() != 1 {
        return None;
    }
    let ArrayExpressionElement::SpreadElement(spread) = &array.elements[0] else {
        return None;
    };
    Some(spread)
}

pub(crate) fn object_length_value<'a>(
    object: &'a oxc_ast::ast::ObjectExpression<'a>,
) -> Option<&'a Expression<'a>> {
    if object.properties.len() != 1 {
        return None;
    }
    let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        return None;
    };
    if property_key_name(&property.key) != Some("length") {
        return None;
    }
    Some(&property.value)
}

pub(crate) fn expression_body<'a>(body: &'a FunctionBody<'a>) -> Option<&'a Expression<'a>> {
    if body.statements.len() != 1 {
        return None;
    }
    let Statement::ExpressionStatement(statement) = &body.statements[0] else {
        return None;
    };
    Some(&statement.expression)
}

pub(crate) fn property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

pub(crate) fn single_expression_statement<'a>(
    statement: &'a Statement<'a>,
) -> Option<&'a oxc_ast::ast::ExpressionStatement<'a>> {
    match statement {
        Statement::ExpressionStatement(expression) => Some(expression),
        Statement::BlockStatement(block) if block.body.len() == 1 => {
            let Statement::ExpressionStatement(expression) = &block.body[0] else {
                return None;
            };
            Some(expression)
        }
        _ => None,
    }
}

pub(crate) fn is_timer_call(call: &CallExpression<'_>) -> bool {
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "setTimeout" | "setInterval")
        }
        Expression::StaticMemberExpression(member)
            if matches!(
                member.object.get_inner_expression(),
                Expression::Identifier(identifier)
                    if matches!(identifier.name.as_str(), "window" | "globalThis")
            ) =>
        {
            matches!(member.property.name.as_str(), "setTimeout" | "setInterval")
        }
        _ => false,
    }
}

#[allow(dead_code)]
fn _new_expression_marker(_: &NewExpression<'_>) {}
