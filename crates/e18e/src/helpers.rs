//! Free helper functions and small private types shared across the rule modules
//! and the AST traversal scanner.

use oxc_ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, CallExpression, Expression, FunctionBody,
    NewExpression, ObjectPropertyKind, PropertyKey, Statement,
};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

use crate::{BanDependency, Diagnostic, DiagnosticData, LineIndex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExprContext {
    Boolean,
    Callee,
    MemberObject,
    Return,
    Statement,
    Other,
}

pub(crate) enum SomeSource<'a> {
    Find(&'a CallExpression<'a>),
    FilterLength(&'a CallExpression<'a>),
}

pub(crate) struct NullishCheck<'a> {
    pub(crate) value: &'a Expression<'a>,
    pub(crate) checks_for_nullish: bool,
}

pub(crate) fn ban_dependency_diagnostic(
    dependency: &BanDependency,
    span: Span,
    source_text: &str,
    line_index: &LineIndex,
) -> Diagnostic {
    let message_id = match dependency.message_id.as_str() {
        "nativeReplacement" => "nativeReplacement",
        "documentedReplacement" => "documentedReplacement",
        "simpleReplacement" => "simpleReplacement",
        "removalReplacement" => "removalReplacement",
        _ => "removalReplacement",
    };
    Diagnostic {
        rule_name: "ban-dependencies",
        message_id,
        data: DiagnosticData {
            name: Some(dependency.module_name.clone()),
            replacement: dependency.replacement.clone(),
            url: dependency.url.clone(),
            description: dependency.description.clone(),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(source_text, span),
        fix: None,
    }
}

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
            single_spread_element(array).map(|spread| &spread.argument)
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

pub(crate) fn is_number_literal(expression: &Expression<'_>, expected: f64) -> bool {
    numeric_literal_value(expression).is_some_and(|value| value == expected)
}

pub(crate) fn numeric_literal_value(expression: &Expression<'_>) -> Option<f64> {
    match expression.get_inner_expression() {
        Expression::NumericLiteral(literal) => Some(literal.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::UnaryNegation => {
            numeric_literal_value(&unary.argument).map(|value| -value)
        }
        _ => None,
    }
}

pub(crate) fn binary_index_of_comparison<'a>(
    left: &'a Expression<'a>,
    right: &'a Expression<'a>,
) -> Option<(&'a CallExpression<'a>, &'a Expression<'a>, bool)> {
    if let Expression::CallExpression(call) = left.get_inner_expression() {
        if is_method_call(call, "indexOf") {
            return Some((call, right, false));
        }
    }
    if let Expression::CallExpression(call) = right.get_inner_expression() {
        if is_method_call(call, "indexOf") {
            return Some((call, left, true));
        }
    }
    None
}

pub(crate) fn normalize_operator(operator: BinaryOperator, reversed: bool) -> BinaryOperator {
    if !reversed {
        return operator;
    }
    match operator {
        BinaryOperator::LessThan => BinaryOperator::GreaterThan,
        BinaryOperator::LessEqualThan => BinaryOperator::GreaterEqualThan,
        BinaryOperator::GreaterThan => BinaryOperator::LessThan,
        BinaryOperator::GreaterEqualThan => BinaryOperator::LessEqualThan,
        other => other,
    }
}

pub(crate) fn includes_negation_for_constant(
    operator: BinaryOperator,
    constant: &Expression<'_>,
) -> Option<bool> {
    if is_number_literal(constant, -1.0) {
        return match operator {
            BinaryOperator::StrictInequality
            | BinaryOperator::Inequality
            | BinaryOperator::GreaterThan => Some(false),
            BinaryOperator::StrictEquality | BinaryOperator::Equality => Some(true),
            _ => None,
        };
    }
    if is_number_literal(constant, 0.0) {
        return match operator {
            BinaryOperator::GreaterEqualThan => Some(false),
            BinaryOperator::LessThan => Some(true),
            _ => None,
        };
    }
    None
}

pub(crate) fn nullish_check<'a>(
    source_text: &str,
    expression: &'a Expression<'a>,
) -> Option<NullishCheck<'a>> {
    match expression.get_inner_expression() {
        Expression::BinaryExpression(binary)
            if matches!(
                binary.operator,
                BinaryOperator::Equality | BinaryOperator::Inequality
            ) && is_null_literal(&binary.right) =>
        {
            Some(NullishCheck {
                value: &binary.left,
                checks_for_nullish: binary.operator == BinaryOperator::Equality,
            })
        }
        Expression::LogicalExpression(logical) => {
            let Expression::BinaryExpression(left) = logical.left.get_inner_expression() else {
                return None;
            };
            let Expression::BinaryExpression(right) = logical.right.get_inner_expression() else {
                return None;
            };
            if source_text[left.left.span().start as usize..left.left.span().end as usize]
                != source_text[right.left.span().start as usize..right.left.span().end as usize]
            {
                return None;
            }
            let pair = (
                is_null_literal(&left.right),
                is_undefined_identifier(&left.right),
                is_null_literal(&right.right),
                is_undefined_identifier(&right.right),
            );
            if !matches!(
                pair,
                (true, false, false, true) | (false, true, true, false)
            ) {
                return None;
            }
            if logical.operator == LogicalOperator::Or
                && left.operator == BinaryOperator::StrictEquality
                && right.operator == BinaryOperator::StrictEquality
            {
                return Some(NullishCheck {
                    value: &left.left,
                    checks_for_nullish: true,
                });
            }
            if logical.operator == LogicalOperator::And
                && left.operator == BinaryOperator::StrictInequality
                && right.operator == BinaryOperator::StrictInequality
            {
                return Some(NullishCheck {
                    value: &left.left,
                    checks_for_nullish: false,
                });
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn is_null_literal(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::NullLiteral(_)
    )
}

pub(crate) fn is_undefined_identifier(expression: &Expression<'_>) -> bool {
    matches!(expression.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "undefined")
}

pub(crate) fn is_null_or_undefined(expression: &Expression<'_>) -> bool {
    is_null_literal(expression) || is_undefined_identifier(expression)
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

pub(crate) fn is_new_date_no_args(new_expression: &NewExpression<'_>) -> bool {
    new_expression.arguments.is_empty()
        && matches!(new_expression.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Date")
}

pub(crate) fn is_regex_expression(expression: Option<&Expression<'_>>) -> bool {
    matches!(
        expression.map(Expression::get_inner_expression),
        Some(Expression::RegExpLiteral(_) | Expression::NewExpression(_))
    )
}

pub(crate) fn static_regexp_args(arguments: &[Argument<'_>]) -> bool {
    if arguments.is_empty() || arguments.len() > 2 {
        return false;
    }
    arguments.iter().all(|argument| {
        matches!(
            argument
                .as_expression()
                .map(Expression::get_inner_expression),
            Some(Expression::StringLiteral(_))
        )
    }) && arguments.get(1).is_none_or(|argument| {
        !argument
            .as_expression()
            .and_then(|expression| match expression.get_inner_expression() {
                Expression::StringLiteral(literal) => Some(literal.value.as_str()),
                _ => None,
            })
            .is_some_and(|flags| flags.contains('g') || flags.contains('y'))
    })
}

pub(crate) fn is_simple_inline_element(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::Identifier(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
}

pub(crate) fn is_safe_from_code_point_arg(argument: &Argument<'_>) -> bool {
    let Some(Expression::NumericLiteral(literal)) = argument
        .as_expression()
        .map(Expression::get_inner_expression)
    else {
        return false;
    };
    literal.value.fract() == 0.0 && (0.0..65536.0).contains(&literal.value)
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

pub(crate) fn format_timer_replacement(
    timer: &str,
    callee: &str,
    delay: &str,
    args: &[String],
) -> String {
    if args.is_empty() {
        format!("{timer}({callee}, {delay})")
    } else {
        format!("{timer}({callee}, {delay}, {})", args.join(", "))
    }
}

pub(crate) fn simple_regex_equivalent(
    regex_text: &str,
) -> Option<(&'static str, &'static str, String)> {
    let inner = regex_text.strip_prefix('/')?;
    let pattern_end = inner.rfind('/')?;
    let (pattern, flags) = inner.split_at(pattern_end);
    let flags = &flags[1..];
    if flags.contains('i') || flags.contains('g') || flags.contains('y') || flags.contains('m') {
        return None;
    }
    if let Some(value) = pattern
        .strip_prefix('^')
        .and_then(|value| value.strip_suffix('$'))
    {
        if is_plain_regex_text(value) {
            return Some(("preferEquals", "===", value.to_owned()));
        }
    }
    if let Some(value) = pattern.strip_prefix('^') {
        if is_plain_regex_text(value) {
            return Some(("preferStartsWith", "startsWith", value.to_owned()));
        }
    }
    if let Some(value) = pattern.strip_suffix('$') {
        if is_plain_regex_text(value) {
            return Some(("preferEndsWith", "endsWith", value.to_owned()));
        }
    }
    if is_plain_regex_text(pattern) {
        return Some(("preferIncludes", "includes", pattern.to_owned()));
    }
    None
}

pub(crate) fn is_plain_regex_text(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(|ch| {
            matches!(
                ch,
                '.' | '*' | '+' | '?' | '[' | ']' | '(' | ')' | '{' | '}' | '|' | '\\'
            )
        })
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

pub(crate) fn find_or_filter_comparison<'a>(
    left: &'a Expression<'a>,
    right: &'a Expression<'a>,
) -> Option<(SomeSource<'a>, &'a Expression<'a>, bool)> {
    if let Some(call) = find_call_or_filter_length(left) {
        return if is_method_call(call, "find") {
            Some((SomeSource::Find(call), right, false))
        } else {
            Some((SomeSource::FilterLength(call), right, false))
        };
    }
    if let Some(call) = find_call_or_filter_length(right) {
        return if is_method_call(call, "find") {
            Some((SomeSource::Find(call), left, true))
        } else {
            Some((SomeSource::FilterLength(call), left, true))
        };
    }
    None
}

pub(crate) fn find_call_or_filter_length<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a CallExpression<'a>> {
    match expression.get_inner_expression() {
        Expression::CallExpression(call) if is_method_call(call, "find") => Some(call),
        Expression::StaticMemberExpression(member) if member.property.name == "length" => {
            let Expression::CallExpression(call) = member.object.get_inner_expression() else {
                return None;
            };
            is_method_call(call, "filter").then_some(call)
        }
        _ => None,
    }
}

pub(crate) fn is_undefined_constant(expression: &Expression<'_>) -> bool {
    is_undefined_identifier(expression)
        || matches!(expression.get_inner_expression(), Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void)
}

pub(crate) fn return_boolean(statement: &oxc_ast::ast::ReturnStatement<'_>, value: bool) -> bool {
    matches!(
        statement.argument.as_ref().map(Expression::get_inner_expression),
        Some(Expression::BooleanLiteral(literal)) if literal.value == value
    )
}
