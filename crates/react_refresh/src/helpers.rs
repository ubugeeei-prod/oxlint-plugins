//! Free helper functions used across the react-refresh scanner.

#![allow(
    unused_imports,
    reason = "Helpers share the react-refresh AST import surface; not every helper uses every type."
)]

use oxc_ast::ast::{
    BindingIdentifier, BindingPattern, Class, ClassElement, Expression, FunctionType,
    IdentifierReference, ModuleExportName, PropertyKey,
};
use oxc_span::GetSpan;

use crate::{is_react_component_name, ComponentCheck, NamedSpan};

pub(crate) fn component_check_for_name(name: &str) -> ComponentCheck {
    if is_react_component_name(name) {
        ComponentCheck::Yes
    } else {
        ComponentCheck::No
    }
}

pub(crate) fn binding_identifier<'a>(identifier: &BindingIdentifier<'a>) -> NamedSpan<'a> {
    NamedSpan {
        name: identifier.name.as_str(),
        span: identifier.span,
    }
}

pub(crate) fn identifier_reference<'a>(identifier: &IdentifierReference<'a>) -> NamedSpan<'a> {
    NamedSpan {
        name: identifier.name.as_str(),
        span: identifier.span,
    }
}

pub(crate) fn binding_pattern_identifier<'a>(pattern: &BindingPattern<'a>) -> Option<NamedSpan<'a>> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(binding_identifier(identifier)),
        _ => None,
    }
}

pub(crate) fn named_export_identifier<'a>(name: &ModuleExportName<'a>) -> Option<NamedSpan<'a>> {
    match name {
        ModuleExportName::IdentifierName(identifier) => Some(NamedSpan {
            name: identifier.name.as_str(),
            span: identifier.span,
        }),
        ModuleExportName::IdentifierReference(identifier) => Some(identifier_reference(identifier)),
        ModuleExportName::StringLiteral(_) => None,
    }
}

pub(crate) fn is_react_class_component(class: &Class<'_>) -> bool {
    let Some(id) = &class.id else {
        return false;
    };
    is_react_component_name(id.name.as_str())
        && class.super_class.is_some()
        && class.body.body.iter().any(|element| match element {
            ClassElement::MethodDefinition(method) => {
                property_key_name(&method.key) == Some("render")
            }
            _ => false,
        })
}

pub(crate) fn property_key_name<'a>(key: &PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn function_param_count(params: &oxc_ast::ast::FormalParameters<'_>) -> usize {
    params.items.len() + usize::from(params.rest.is_some())
}

pub(crate) fn is_constant_export_expression(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
    )
}

pub(crate) fn is_create_context_call(expression: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expression.get_inner_expression() else {
        return false;
    };
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => identifier.name == "createContext",
        Expression::StaticMemberExpression(member) => member.property.name == "createContext",
        _ => false,
    }
}
