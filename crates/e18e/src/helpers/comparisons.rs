//! Binary/logical comparison helpers (indexOf, nullish, find/filter length).

use oxc_ast::ast::{CallExpression, Expression};
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};

use crate::helpers::ast::is_method_call;
use crate::helpers::literals::{is_null_literal, is_number_literal, is_undefined_identifier};
use crate::helpers::{NullishCheck, SomeSource};

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
