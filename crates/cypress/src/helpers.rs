//! Free helper functions used across the cypress scanner.

#![allow(
    unused_imports,
    reason = "Helpers share the cypress AST import surface; not every helper uses every type."
)]

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, CallExpression, ChainElement, Class, ClassElement,
    Declaration, ExportDefaultDeclarationKind, Expression, FunctionBody, ObjectPropertyKind,
    PropertyKey, Statement,
};
use oxlint_plugins_carton::SmallVec;

pub(crate) fn call_static_member_name<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    match call.callee.get_inner_expression() {
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

pub(crate) fn call_static_member_object<'a>(call: &'a CallExpression<'a>) -> Option<&'a Expression<'a>> {
    match call.callee.get_inner_expression() {
        Expression::StaticMemberExpression(member) => Some(&member.object),
        _ => None,
    }
}

pub(crate) fn previous_command_in_chain<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    let Some(Expression::CallExpression(object_call)) = call_static_member_object(call) else {
        return None;
    };
    call_static_member_name(object_call)
}

pub(crate) fn expression_identifier_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn callee_identifier_name<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn cypress_command_names<'a>(call: &'a CallExpression<'a>) -> Option<(&'a str, &'a str)> {
    let mut names = SmallVec::<[&'a str; 8]>::new();
    collect_cypress_command_names(call, &mut names)?;
    let first = *names.first()?;
    let last = *names.last()?;
    Some((first, last))
}

pub(crate) fn collect_cypress_command_names<'a>(
    call: &'a CallExpression<'a>,
    names: &mut SmallVec<[&'a str; 8]>,
) -> Option<()> {
    let command = call_static_member_name(call)?;
    let object = call_static_member_object(call)?;

    if expression_identifier_name(object) == Some("cy") {
        names.push(command);
        return Some(());
    }

    if let Expression::CallExpression(object_call) = object.get_inner_expression() {
        collect_cypress_command_names(object_call, names)?;
        names.push(command);
        return Some(());
    }

    None
}

pub(crate) fn call_has_force_option<'a>(call: &'a CallExpression<'a>) -> bool {
    call.arguments.iter().any(|argument| {
        let Argument::ObjectExpression(object) = argument else {
            return false;
        };
        object.properties.iter().any(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            property_key_name(&property.key) == Some("force")
        })
    })
}

pub(crate) fn property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        PropertyKey::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn is_alias_or_data_selector(selector: &str) -> bool {
    selector.starts_with("[data-") || selector.starts_with('@')
}

pub(crate) fn argument_is_async_function(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::ArrowFunctionExpression(function) => function.r#async,
        Argument::FunctionExpression(function) => function.r#async,
        _ => false,
    }
}

pub(crate) fn argument_contains_cypress_identifier(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        Argument::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        _ => false,
    }
}

pub(crate) fn function_body_contains_cypress(body: &FunctionBody<'_>) -> bool {
    body.statements.iter().any(statement_contains_cypress)
}

pub(crate) fn statement_contains_cypress(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => {
            expression_contains_cypress(&statement.expression)
        }
        Statement::BlockStatement(block) => block.body.iter().any(statement_contains_cypress),
        Statement::IfStatement(statement) => {
            expression_contains_cypress(&statement.test)
                || statement_contains_cypress(&statement.consequent)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(statement_contains_cypress)
        }
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                declarator
                    .init
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
            })
        }
        Statement::ReturnStatement(statement) => statement
            .argument
            .as_ref()
            .is_some_and(expression_contains_cypress),
        Statement::ThrowStatement(statement) => expression_contains_cypress(&statement.argument),
        Statement::WhileStatement(statement) => {
            expression_contains_cypress(&statement.test)
                || statement_contains_cypress(&statement.body)
        }
        Statement::DoWhileStatement(statement) => {
            statement_contains_cypress(&statement.body)
                || expression_contains_cypress(&statement.test)
        }
        Statement::ForStatement(statement) => {
            statement
                .test
                .as_ref()
                .is_some_and(expression_contains_cypress)
                || statement
                    .update
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
                || statement_contains_cypress(&statement.body)
        }
        Statement::ForInStatement(statement) => {
            expression_contains_cypress(&statement.right)
                || statement_contains_cypress(&statement.body)
        }
        Statement::ForOfStatement(statement) => {
            expression_contains_cypress(&statement.right)
                || statement_contains_cypress(&statement.body)
        }
        Statement::SwitchStatement(statement) => {
            expression_contains_cypress(&statement.discriminant)
                || statement.cases.iter().any(|case| {
                    case.test.as_ref().is_some_and(expression_contains_cypress)
                        || case.consequent.iter().any(statement_contains_cypress)
                })
        }
        Statement::TryStatement(statement) => {
            statement.block.body.iter().any(statement_contains_cypress)
                || statement
                    .handler
                    .as_ref()
                    .is_some_and(|handler| handler.body.body.iter().any(statement_contains_cypress))
                || statement
                    .finalizer
                    .as_ref()
                    .is_some_and(|finalizer| finalizer.body.iter().any(statement_contains_cypress))
        }
        Statement::FunctionDeclaration(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Statement::ClassDeclaration(class) => class_contains_cypress(class),
        Statement::ExportNamedDeclaration(declaration) => declaration
            .declaration
            .as_ref()
            .is_some_and(declaration_contains_cypress),
        Statement::ExportDefaultDeclaration(declaration) => match &declaration.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => function
                .body
                .as_deref()
                .is_some_and(function_body_contains_cypress),
            ExportDefaultDeclarationKind::ClassDeclaration(class) => class_contains_cypress(class),
            declaration => declaration
                .as_expression()
                .is_some_and(expression_contains_cypress),
        },
        _ => false,
    }
}

pub(crate) fn declaration_contains_cypress(declaration: &Declaration<'_>) -> bool {
    match declaration {
        Declaration::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                declarator
                    .init
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
            })
        }
        Declaration::FunctionDeclaration(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Declaration::ClassDeclaration(class) => class_contains_cypress(class),
        _ => false,
    }
}

pub(crate) fn class_contains_cypress(class: &Class<'_>) -> bool {
    class.body.body.iter().any(|element| match element {
        ClassElement::StaticBlock(block) => block.body.iter().any(statement_contains_cypress),
        ClassElement::MethodDefinition(method) => method
            .value
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        ClassElement::PropertyDefinition(property) => property
            .value
            .as_ref()
            .is_some_and(expression_contains_cypress),
        ClassElement::AccessorProperty(property) => property
            .value
            .as_ref()
            .is_some_and(expression_contains_cypress),
        ClassElement::TSIndexSignature(_) => false,
    })
}

pub(crate) fn expression_contains_cypress(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "cy" | "Cypress")
        }
        Expression::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        Expression::StaticMemberExpression(member) => expression_contains_cypress(&member.object),
        Expression::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        Expression::ChainExpression(chain) => chain_element_contains_cypress(&chain.expression),
        Expression::ParenthesizedExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSAsExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSSatisfiesExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSTypeAssertion(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSNonNullExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::TSInstantiationExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        Expression::AwaitExpression(expression) => {
            expression_contains_cypress(&expression.argument)
        }
        Expression::ArrayExpression(expression) => expression
            .elements
            .iter()
            .any(array_element_contains_cypress),
        Expression::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    expression_contains_cypress(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    expression_contains_cypress(&spread.argument)
                }
            })
        }
        Expression::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        Expression::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Expression::ClassExpression(class) => class_contains_cypress(class),
        Expression::AssignmentExpression(expression) => {
            expression_contains_cypress(&expression.right)
        }
        Expression::ConditionalExpression(expression) => {
            expression_contains_cypress(&expression.test)
                || expression_contains_cypress(&expression.consequent)
                || expression_contains_cypress(&expression.alternate)
        }
        Expression::BinaryExpression(expression) => {
            expression_contains_cypress(&expression.left)
                || expression_contains_cypress(&expression.right)
        }
        Expression::LogicalExpression(expression) => {
            expression_contains_cypress(&expression.left)
                || expression_contains_cypress(&expression.right)
        }
        Expression::SequenceExpression(expression) => expression
            .expressions
            .iter()
            .any(expression_contains_cypress),
        Expression::UnaryExpression(expression) => {
            expression_contains_cypress(&expression.argument)
        }
        Expression::YieldExpression(expression) => expression
            .argument
            .as_ref()
            .is_some_and(expression_contains_cypress),
        Expression::TaggedTemplateExpression(expression) => {
            expression_contains_cypress(&expression.tag)
                || expression
                    .quasi
                    .expressions
                    .iter()
                    .any(expression_contains_cypress)
        }
        Expression::TemplateLiteral(template) => {
            template.expressions.iter().any(expression_contains_cypress)
        }
        Expression::ImportExpression(expression) => {
            expression_contains_cypress(&expression.source)
                || expression
                    .options
                    .as_ref()
                    .is_some_and(expression_contains_cypress)
        }
        _ => false,
    }
}

pub(crate) fn chain_element_contains_cypress(element: &ChainElement<'_>) -> bool {
    match element {
        ChainElement::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        ChainElement::StaticMemberExpression(member) => expression_contains_cypress(&member.object),
        ChainElement::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        ChainElement::TSNonNullExpression(expression) => {
            expression_contains_cypress(&expression.expression)
        }
        ChainElement::PrivateFieldExpression(member) => expression_contains_cypress(&member.object),
    }
}

pub(crate) fn argument_expression_contains_cypress(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::SpreadElement(spread) => expression_contains_cypress(&spread.argument),
        Argument::Identifier(identifier) => matches!(identifier.name.as_str(), "cy" | "Cypress"),
        Argument::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        Argument::StaticMemberExpression(member) => expression_contains_cypress(&member.object),
        Argument::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        Argument::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        Argument::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        Argument::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    expression_contains_cypress(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    expression_contains_cypress(&spread.argument)
                }
            })
        }
        Argument::ArrayExpression(expression) => expression
            .elements
            .iter()
            .any(array_element_contains_cypress),
        Argument::ConditionalExpression(expression) => {
            expression_contains_cypress(&expression.test)
                || expression_contains_cypress(&expression.consequent)
                || expression_contains_cypress(&expression.alternate)
        }
        Argument::AwaitExpression(expression) => expression_contains_cypress(&expression.argument),
        Argument::SequenceExpression(expression) => expression
            .expressions
            .iter()
            .any(expression_contains_cypress),
        Argument::TaggedTemplateExpression(expression) => {
            expression_contains_cypress(&expression.tag)
                || expression
                    .quasi
                    .expressions
                    .iter()
                    .any(expression_contains_cypress)
        }
        Argument::TemplateLiteral(template) => {
            template.expressions.iter().any(expression_contains_cypress)
        }
        _ => false,
    }
}

pub(crate) fn array_element_contains_cypress(element: &ArrayExpressionElement<'_>) -> bool {
    match element {
        ArrayExpressionElement::SpreadElement(spread) => {
            expression_contains_cypress(&spread.argument)
        }
        ArrayExpressionElement::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "cy" | "Cypress")
        }
        ArrayExpressionElement::CallExpression(call) => {
            expression_contains_cypress(&call.callee)
                || call
                    .arguments
                    .iter()
                    .any(argument_expression_contains_cypress)
        }
        ArrayExpressionElement::StaticMemberExpression(member) => {
            expression_contains_cypress(&member.object)
        }
        ArrayExpressionElement::ComputedMemberExpression(member) => {
            expression_contains_cypress(&member.object)
                || expression_contains_cypress(&member.expression)
        }
        ArrayExpressionElement::ArrowFunctionExpression(function) => {
            function_body_contains_cypress(&function.body)
        }
        ArrayExpressionElement::FunctionExpression(function) => function
            .body
            .as_deref()
            .is_some_and(function_body_contains_cypress),
        ArrayExpressionElement::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    expression_contains_cypress(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    expression_contains_cypress(&spread.argument)
                }
            })
        }
        ArrayExpressionElement::ArrayExpression(expression) => expression
            .elements
            .iter()
            .any(array_element_contains_cypress),
        ArrayExpressionElement::ConditionalExpression(expression) => {
            expression_contains_cypress(&expression.test)
                || expression_contains_cypress(&expression.consequent)
                || expression_contains_cypress(&expression.alternate)
        }
        ArrayExpressionElement::Elision(_) => false,
        _ => false,
    }
}
