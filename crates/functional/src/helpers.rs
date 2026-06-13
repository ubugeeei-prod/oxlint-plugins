//! Free helper functions used across the functional scanner.

#![allow(
    unused_imports,
    reason = "Helpers share the functional AST import surface; not every helper uses every type."
)]

use oxc_ast::ast::*;

pub(crate) fn assignment_target_is_member(target: &AssignmentTarget<'_>) -> bool {
    matches!(
        target,
        AssignmentTarget::StaticMemberExpression(_)
            | AssignmentTarget::ComputedMemberExpression(_)
            | AssignmentTarget::PrivateFieldExpression(_)
    )
}

pub(crate) fn is_identifier_expression(expression: &Expression<'_>, name: &str) -> bool {
    matches!(expression.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == name)
}

pub(crate) fn is_static_call(call: &CallExpression<'_>, object_name: &str, method_name: &str) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    member.property.name == method_name && is_identifier_expression(&member.object, object_name)
}

pub(crate) fn is_mutating_call(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    let method = member.property.name.as_str();
    matches!(
        method,
        "copyWithin"
            | "fill"
            | "pop"
            | "push"
            | "reverse"
            | "shift"
            | "sort"
            | "splice"
            | "unshift"
            | "clear"
            | "delete"
            | "set"
            | "add"
            | "assign"
            | "defineProperties"
            | "defineProperty"
            | "setPrototypeOf"
    )
}

pub(crate) fn type_reference_name<'a>(reference: &'a TSTypeReference<'a>) -> Option<&'a str> {
    match &reference.type_name {
        TSTypeName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        TSTypeName::QualifiedName(qualified) => Some(qualified.right.name.as_str()),
        TSTypeName::ThisExpression(_) => None,
    }
}

pub(crate) fn is_mutable_collection_name(name: &str) -> bool {
    matches!(name, "Array" | "Map" | "Set" | "WeakMap" | "WeakSet")
}

pub(crate) fn is_mutable_type(ty: &TSType<'_>) -> bool {
    match ty {
        TSType::TSArrayType(_) | TSType::TSTupleType(_) => true,
        TSType::TSTypeReference(reference) => {
            type_reference_name(reference).is_some_and(is_mutable_collection_name)
        }
        TSType::TSTypeLiteral(literal) => interface_has_mutable_members(&literal.members),
        TSType::TSUnionType(union) => union.types.iter().any(is_mutable_type),
        TSType::TSIntersectionType(intersection) => intersection.types.iter().any(is_mutable_type),
        TSType::TSParenthesizedType(parenthesized) => {
            is_mutable_type(&parenthesized.type_annotation)
        }
        TSType::TSTypeOperatorType(operator) => {
            operator.operator != TSTypeOperatorOperator::Readonly
                && is_mutable_type(&operator.type_annotation)
        }
        _ => false,
    }
}

pub(crate) fn signature_is_function_like(signature: &TSSignature<'_>) -> bool {
    match signature {
        TSSignature::TSMethodSignature(_)
        | TSSignature::TSCallSignatureDeclaration(_)
        | TSSignature::TSConstructSignatureDeclaration(_) => true,
        TSSignature::TSPropertySignature(property) => {
            property.type_annotation.as_ref().is_some_and(|annotation| {
                matches!(annotation.type_annotation, TSType::TSFunctionType(_))
            })
        }
        TSSignature::TSIndexSignature(_) => false,
    }
}

pub(crate) fn has_mixed_signatures(signatures: &[TSSignature<'_>]) -> bool {
    if signatures.len() < 2 {
        return false;
    }
    let first = signature_is_function_like(&signatures[0]);
    signatures
        .iter()
        .skip(1)
        .any(|signature| signature_is_function_like(signature) != first)
}

pub(crate) fn interface_has_mutable_members(signatures: &[TSSignature<'_>]) -> bool {
    signatures.iter().any(|signature| match signature {
        TSSignature::TSPropertySignature(property) => {
            !property.readonly
                || property
                    .type_annotation
                    .as_ref()
                    .is_some_and(|annotation| is_mutable_type(&annotation.type_annotation))
        }
        TSSignature::TSIndexSignature(signature) => {
            !signature.readonly || is_mutable_type(&signature.type_annotation.type_annotation)
        }
        _ => false,
    })
}

pub(crate) fn single_returned_call<'a>(body: &'a FunctionBody<'a>) -> Option<&'a CallExpression<'a>> {
    if body.statements.len() != 1 {
        return None;
    }
    match &body.statements[0] {
        Statement::ExpressionStatement(statement) => {
            match statement.expression.get_inner_expression() {
                Expression::CallExpression(call) => Some(&**call),
                _ => None,
            }
        }
        Statement::ReturnStatement(statement) => statement.argument.as_ref().and_then(|argument| {
            if let Expression::CallExpression(call) = argument.get_inner_expression() {
                Some(&**call)
            } else {
                None
            }
        }),
        _ => None,
    }
}

pub(crate) fn call_arguments_match_params(call: &CallExpression<'_>, params: &FormalParameters<'_>) -> bool {
    if call.arguments.len() != params.items.len() {
        return false;
    }
    call.arguments
        .iter()
        .zip(params.items.iter())
        .all(|(argument, param)| {
            let Argument::Identifier(argument) = argument else {
                return false;
            };
            let BindingPattern::BindingIdentifier(parameter) = &param.pattern else {
                return false;
            };
            argument.name == parameter.name
        })
}
