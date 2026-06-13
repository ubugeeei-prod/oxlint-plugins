//! Free helper functions used across the react-hooks scanner.

#![allow(
    unused_imports,
    reason = "Helpers share the react-hooks AST import surface; not every helper uses every type."
)]

use oxc_ast::ast::{AssignmentTarget, BindingPattern, Expression, Function, PropertyKey};

use crate::is_react_component_name;

pub(crate) fn binding_pattern_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn function_name<'a>(function: &'a Function<'a>) -> Option<&'a str> {
    function
        .id
        .as_ref()
        .map(|identifier| identifier.name.as_str())
}

pub(crate) fn method_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::PrivateIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn assignment_target_name<'a>(target: &'a AssignmentTarget<'a>) -> Option<&'a str> {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(identifier) => Some(identifier.name.as_str()),
        AssignmentTarget::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

pub(crate) fn object_is_pascal_case_identifier(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::Identifier(identifier) if is_react_component_name(identifier.name.as_str())
    )
}

pub(crate) fn is_component_callback_callee(expression: &Expression<'_>) -> bool {
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
