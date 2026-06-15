//! Free helper functions used across the mocha scanner.

#![allow(
    unused_imports,
    reason = "Helpers share the mocha AST import surface; not every helper uses every type."
)]

use std::fmt::{Arguments, Write as _};

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, BindingPattern, CallExpression, ChainElement, Expression,
    PropertyKey, Statement,
};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{Callback, CallbackBody, EntityType, MochaInterface, Modifier};

pub(crate) fn direct_statement_mocha_span(statement: &Statement<'_>) -> Option<Span> {
    match statement {
        Statement::ExpressionStatement(statement) => {
            direct_expression_mocha_span(&statement.expression)
        }
        _ => None,
    }
}

pub(crate) fn direct_expression_mocha_span(expression: &Expression<'_>) -> Option<Span> {
    match expression.get_inner_expression() {
        Expression::CallExpression(call) => {
            if call_path(call)
                .as_deref()
                .and_then(classify_mocha_path)
                .is_some()
            {
                return Some(call.span);
            }
            direct_expression_mocha_span(&call.callee)
                .or_else(|| call.arguments.iter().find_map(direct_argument_mocha_span))
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => {
                if call_path(call)
                    .as_deref()
                    .and_then(classify_mocha_path)
                    .is_some()
                {
                    Some(call.span)
                } else {
                    direct_expression_mocha_span(&call.callee)
                }
            }
            ChainElement::StaticMemberExpression(member) => {
                direct_expression_mocha_span(&member.object)
            }
            _ => None,
        },
        Expression::StaticMemberExpression(member) => direct_expression_mocha_span(&member.object),
        Expression::ComputedMemberExpression(member) => {
            direct_expression_mocha_span(&member.object)
        }
        _ => None,
    }
}

pub(crate) fn direct_argument_mocha_span(argument: &Argument<'_>) -> Option<Span> {
    match argument {
        Argument::CallExpression(call) => {
            if call_path(call)
                .as_deref()
                .and_then(classify_mocha_path)
                .is_some()
            {
                Some(call.span)
            } else {
                direct_expression_mocha_span(&call.callee)
            }
        }
        Argument::StaticMemberExpression(member) => direct_expression_mocha_span(&member.object),
        Argument::ComputedMemberExpression(member) => direct_expression_mocha_span(&member.object),
        _ => None,
    }
}

pub(crate) fn compact_format(args: Arguments<'_>) -> CompactString {
    let mut message = CompactString::new("");
    let _ = message.write_fmt(args);
    message
}

pub(crate) fn display_call_name(name: &str) -> CompactString {
    compact_format(format_args!("{name}()"))
}

pub(crate) fn classify_mocha_path(
    path: &[&str],
) -> Option<(&'static str, EntityType, MochaInterface, Option<Modifier>)> {
    let first = *path.first()?;
    let last = *path.last()?;
    let modifier = if last == "only" {
        Some(Modifier::Exclusive)
    } else if last == "skip" {
        Some(Modifier::Pending)
    } else {
        None
    };
    let base = if path.len() == 1 || (modifier.is_some() && path.len() == 2) {
        first
    } else {
        return None;
    };

    match base {
        "describe" => Some(("describe", EntityType::Suite, MochaInterface::Bdd, modifier)),
        "context" => Some(("context", EntityType::Suite, MochaInterface::Bdd, modifier)),
        "suite" => Some(("suite", EntityType::Suite, MochaInterface::Tdd, modifier)),
        "it" => Some(("it", EntityType::TestCase, MochaInterface::Bdd, modifier)),
        "specify" => Some((
            "specify",
            EntityType::TestCase,
            MochaInterface::Bdd,
            modifier,
        )),
        "test" => Some(("test", EntityType::TestCase, MochaInterface::Tdd, modifier)),
        "before" | "after" | "beforeEach" | "afterEach" => {
            let hook = match base {
                "before" => "before",
                "after" => "after",
                "beforeEach" => "beforeEach",
                _ => "afterEach",
            };
            Some((hook, EntityType::Hook, MochaInterface::Bdd, None))
        }
        "suiteSetup" | "suiteTeardown" | "setup" | "teardown" => {
            let hook = match base {
                "suiteSetup" => "suiteSetup",
                "suiteTeardown" => "suiteTeardown",
                "setup" => "setup",
                _ => "teardown",
            };
            Some((hook, EntityType::Hook, MochaInterface::Tdd, None))
        }
        "xdescribe" => Some((
            "xdescribe",
            EntityType::Suite,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        "xcontext" => Some((
            "xcontext",
            EntityType::Suite,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        "xit" => Some((
            "xit",
            EntityType::TestCase,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        "xspecify" => Some((
            "xspecify",
            EntityType::TestCase,
            MochaInterface::Bdd,
            Some(Modifier::Pending),
        )),
        _ => None,
    }
}

pub(crate) fn call_path<'a>(call: &'a CallExpression<'a>) -> Option<SmallVec<[&'a str; 3]>> {
    let mut path = SmallVec::new();
    collect_callee_path(call.callee.get_inner_expression(), &mut path)?;
    Some(path)
}

pub(crate) fn collect_callee_path<'a>(
    expression: &'a Expression<'a>,
    path: &mut SmallVec<[&'a str; 3]>,
) -> Option<()> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => {
            path.push(identifier.name.as_str());
            Some(())
        }
        Expression::StaticMemberExpression(member) => {
            collect_callee_path(&member.object, path)?;
            path.push(member.property.name.as_str());
            Some(())
        }
        _ => None,
    }
}

pub(crate) fn callback_from_argument<'a>(argument: &'a Argument<'a>) -> Option<Callback<'a>> {
    match argument {
        Argument::FunctionExpression(function) => {
            let body = function.body.as_deref()?;
            Some(Callback {
                span: function.span,
                body: CallbackBody::Function(body),
                async_function: function.r#async,
                arrow: false,
                named_function: function.id.is_some(),
                params_len: function.params.items.len(),
                first_param_name: function
                    .params
                    .items
                    .first()
                    .and_then(|param| binding_identifier_name(&param.pattern)),
            })
        }
        Argument::ArrowFunctionExpression(function) => Some(Callback {
            span: function.span,
            body: CallbackBody::Function(&function.body),
            async_function: function.r#async,
            arrow: true,
            named_function: false,
            params_len: function.params.items.len(),
            first_param_name: function
                .params
                .items
                .first()
                .and_then(|param| binding_identifier_name(&param.pattern)),
        }),
        _ => None,
    }
}

pub(crate) fn binding_identifier_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn argument_string_value<'a>(argument: &'a Argument<'a>) -> Option<&'a str> {
    match argument {
        Argument::StringLiteral(literal) => Some(literal.value.as_str()),
        Argument::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .map(|value| value.as_str()),
        _ => None,
    }
}

pub(crate) fn is_suite_config_call(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    matches!(
        member.object.get_inner_expression(),
        Expression::ThisExpression(_)
    ) && matches!(
        member.property.name.as_str(),
        "timeout" | "slow" | "retries"
    )
}

pub(crate) fn callback_body_calls_identifier(body: CallbackBody<'_>, name: &str) -> bool {
    match body {
        CallbackBody::Function(body) => body
            .statements
            .iter()
            .any(|statement| statement_calls_identifier(statement, name)),
    }
}

pub(crate) fn callback_body_returns_value(body: CallbackBody<'_>) -> bool {
    match body {
        CallbackBody::Function(body) => body.statements.iter().any(statement_returns_value),
    }
}

pub(crate) fn callback_body_returns_promise(body: CallbackBody<'_>) -> bool {
    match body {
        CallbackBody::Function(body) => body.statements.iter().any(|statement| {
            matches!(statement, Statement::ReturnStatement(statement) if statement.argument.as_ref().is_some_and(non_literal_expression))
        }),
    }
}

pub(crate) fn callback_body_contains_this(body: CallbackBody<'_>) -> bool {
    match body {
        CallbackBody::Function(body) => body.statements.iter().any(statement_contains_this),
    }
}

pub(crate) fn statement_returns_value(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ReturnStatement(statement) => statement.argument.is_some(),
        Statement::BlockStatement(block) => block.body.iter().any(statement_returns_value),
        Statement::IfStatement(statement) => {
            statement_returns_value(&statement.consequent)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| statement_returns_value(alternate))
        }
        _ => false,
    }
}

pub(crate) fn non_literal_expression(expression: &Expression<'_>) -> bool {
    !matches!(
        expression.get_inner_expression(),
        Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::RegExpLiteral(_)
    )
}

pub(crate) fn statement_contains_this(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => {
            expression_contains_this(&statement.expression)
        }
        Statement::BlockStatement(block) => block.body.iter().any(statement_contains_this),
        Statement::ReturnStatement(statement) => statement
            .argument
            .as_ref()
            .is_some_and(expression_contains_this),
        Statement::IfStatement(statement) => {
            expression_contains_this(&statement.test)
                || statement_contains_this(&statement.consequent)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| statement_contains_this(alternate))
        }
        Statement::ThrowStatement(statement) => expression_contains_this(&statement.argument),
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                declarator
                    .init
                    .as_ref()
                    .is_some_and(expression_contains_this)
            })
        }
        Statement::WhileStatement(statement) => {
            expression_contains_this(&statement.test) || statement_contains_this(&statement.body)
        }
        Statement::DoWhileStatement(statement) => {
            statement_contains_this(&statement.body) || expression_contains_this(&statement.test)
        }
        Statement::ForStatement(statement) => {
            statement
                .test
                .as_ref()
                .is_some_and(expression_contains_this)
                || statement
                    .update
                    .as_ref()
                    .is_some_and(expression_contains_this)
                || statement_contains_this(&statement.body)
        }
        Statement::ForInStatement(statement) => {
            expression_contains_this(&statement.right) || statement_contains_this(&statement.body)
        }
        Statement::ForOfStatement(statement) => {
            expression_contains_this(&statement.right) || statement_contains_this(&statement.body)
        }
        Statement::SwitchStatement(statement) => {
            expression_contains_this(&statement.discriminant)
                || statement.cases.iter().any(|case| {
                    case.test.as_ref().is_some_and(expression_contains_this)
                        || case.consequent.iter().any(statement_contains_this)
                })
        }
        Statement::TryStatement(statement) => {
            statement.block.body.iter().any(statement_contains_this)
                || statement
                    .handler
                    .as_ref()
                    .is_some_and(|handler| handler.body.body.iter().any(statement_contains_this))
                || statement
                    .finalizer
                    .as_ref()
                    .is_some_and(|finalizer| finalizer.body.iter().any(statement_contains_this))
        }
        _ => false,
    }
}

pub(crate) fn expression_contains_this(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::ThisExpression(_) => true,
        Expression::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => {
                expression_contains_this(&call.callee)
                    || call.arguments.iter().any(argument_contains_this)
            }
            ChainElement::StaticMemberExpression(member) => {
                expression_contains_this(&member.object)
            }
            ChainElement::ComputedMemberExpression(member) => {
                expression_contains_this(&member.object)
                    || expression_contains_this(&member.expression)
            }
            _ => false,
        },
        Expression::StaticMemberExpression(member) => expression_contains_this(&member.object),
        Expression::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        Expression::UnaryExpression(expression) => expression_contains_this(&expression.argument),
        Expression::AwaitExpression(expression) => expression_contains_this(&expression.argument),
        Expression::BinaryExpression(expression) => {
            expression_contains_this(&expression.left)
                || expression_contains_this(&expression.right)
        }
        Expression::LogicalExpression(expression) => {
            expression_contains_this(&expression.left)
                || expression_contains_this(&expression.right)
        }
        Expression::ConditionalExpression(expression) => {
            expression_contains_this(&expression.test)
                || expression_contains_this(&expression.consequent)
                || expression_contains_this(&expression.alternate)
        }
        Expression::AssignmentExpression(expression) => expression_contains_this(&expression.right),
        Expression::SequenceExpression(expression) => {
            expression.expressions.iter().any(expression_contains_this)
        }
        Expression::TemplateLiteral(template) => {
            template.expressions.iter().any(expression_contains_this)
        }
        Expression::TaggedTemplateExpression(expression) => {
            expression_contains_this(&expression.tag)
                || expression
                    .quasi
                    .expressions
                    .iter()
                    .any(expression_contains_this)
        }
        Expression::ArrayExpression(expression) => {
            expression.elements.iter().any(array_element_contains_this)
        }
        Expression::ObjectExpression(expression) => expression.properties.iter().any(|property| {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                (property.computed && property_key_contains_this(&property.key))
                    || expression_contains_this(&property.value)
            } else {
                false
            }
        }),
        _ => false,
    }
}

pub(crate) fn argument_contains_this(argument: &Argument<'_>) -> bool {
    match argument {
        Argument::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        Argument::StaticMemberExpression(member) => expression_contains_this(&member.object),
        Argument::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        Argument::ArrayExpression(expression) => {
            expression.elements.iter().any(array_element_contains_this)
        }
        Argument::ObjectExpression(expression) => expression.properties.iter().any(|property| {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                expression_contains_this(&property.value)
            } else {
                false
            }
        }),
        Argument::ConditionalExpression(expression) => {
            expression_contains_this(&expression.test)
                || expression_contains_this(&expression.consequent)
                || expression_contains_this(&expression.alternate)
        }
        _ => false,
    }
}

pub(crate) fn array_element_contains_this(element: &ArrayExpressionElement<'_>) -> bool {
    match element {
        ArrayExpressionElement::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        ArrayExpressionElement::StaticMemberExpression(member) => {
            expression_contains_this(&member.object)
        }
        ArrayExpressionElement::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        ArrayExpressionElement::ArrayExpression(expression) => {
            expression.elements.iter().any(array_element_contains_this)
        }
        ArrayExpressionElement::ObjectExpression(expression) => {
            expression.properties.iter().any(|property| {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property {
                    expression_contains_this(&property.value)
                } else {
                    false
                }
            })
        }
        ArrayExpressionElement::ConditionalExpression(expression) => {
            expression_contains_this(&expression.test)
                || expression_contains_this(&expression.consequent)
                || expression_contains_this(&expression.alternate)
        }
        _ => false,
    }
}

pub(crate) fn property_key_contains_this(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::CallExpression(call) => {
            expression_contains_this(&call.callee)
                || call.arguments.iter().any(argument_contains_this)
        }
        PropertyKey::StaticMemberExpression(member) => expression_contains_this(&member.object),
        PropertyKey::ComputedMemberExpression(member) => {
            expression_contains_this(&member.object) || expression_contains_this(&member.expression)
        }
        _ => false,
    }
}

pub(crate) fn statement_calls_identifier(statement: &Statement<'_>, name: &str) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => {
            expression_calls_identifier(&statement.expression, name)
        }
        Statement::BlockStatement(block) => block
            .body
            .iter()
            .any(|statement| statement_calls_identifier(statement, name)),
        Statement::ReturnStatement(statement) => statement
            .argument
            .as_ref()
            .is_some_and(|argument| expression_calls_identifier(argument, name)),
        Statement::IfStatement(statement) => {
            expression_calls_identifier(&statement.test, name)
                || statement_calls_identifier(&statement.consequent, name)
                || statement
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| statement_calls_identifier(alternate, name))
        }
        _ => false,
    }
}

pub(crate) fn expression_calls_identifier(expression: &Expression<'_>, name: &str) -> bool {
    match expression.get_inner_expression() {
        Expression::CallExpression(call) => {
            matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name.as_str() == name)
                || call
                    .arguments
                    .iter()
                    .any(|argument| argument_calls_identifier(argument, name))
        }
        Expression::StaticMemberExpression(member) => {
            expression_calls_identifier(&member.object, name)
        }
        Expression::ComputedMemberExpression(member) => {
            expression_calls_identifier(&member.object, name)
                || expression_calls_identifier(&member.expression, name)
        }
        Expression::BinaryExpression(expression) => {
            expression_calls_identifier(&expression.left, name)
                || expression_calls_identifier(&expression.right, name)
        }
        Expression::LogicalExpression(expression) => {
            expression_calls_identifier(&expression.left, name)
                || expression_calls_identifier(&expression.right, name)
        }
        Expression::ConditionalExpression(expression) => {
            expression_calls_identifier(&expression.test, name)
                || expression_calls_identifier(&expression.consequent, name)
                || expression_calls_identifier(&expression.alternate, name)
        }
        _ => false,
    }
}

pub(crate) fn argument_calls_identifier(argument: &Argument<'_>, name: &str) -> bool {
    match argument {
        Argument::CallExpression(call) => {
            matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name.as_str() == name)
        }
        Argument::Identifier(identifier) => identifier.name.as_str() == name,
        _ => false,
    }
}
