//! Literal/identifier predicates and small string helpers used across rules.

use oxc_ast::ast::{Argument, Expression, NewExpression, ReturnStatement};
use oxc_syntax::operator::UnaryOperator;

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

pub(crate) fn is_undefined_constant(expression: &Expression<'_>) -> bool {
    is_undefined_identifier(expression)
        || matches!(expression.get_inner_expression(), Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void)
}

pub(crate) fn return_boolean(statement: &ReturnStatement<'_>, value: bool) -> bool {
    matches!(
        statement.argument.as_ref().map(Expression::get_inner_expression),
        Some(Expression::BooleanLiteral(literal)) if literal.value == value
    )
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
