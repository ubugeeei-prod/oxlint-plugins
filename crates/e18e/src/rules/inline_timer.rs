//! Rule implementation methods, attached to [`Scanner`] via additional
//! `impl` blocks. The traversal in `scanner.rs` dispatches into these.

#![allow(
    unused_imports,
    reason = "Rule modules share a common import surface for readability; not every rule uses every helper."
)]

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, CallExpression, Expression, ImportDeclaration, NewExpression,
    ObjectPropertyKind, Statement,
};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{
    ExprContext, SomeSource, ban_dependency_diagnostic, binary_index_of_comparison, callee_path,
    constant_callback_value, copy_pattern_optional, copy_pattern_source, expression_body,
    expression_contains_spread, find_call_or_filter_length, find_or_filter_comparison,
    format_timer_replacement, function_body_contains_spread, includes_negation_for_constant,
    is_constant_expression, is_method_call, is_new_date_no_args, is_null_literal,
    is_null_or_undefined, is_number_literal, is_plain_regex_text, is_regex_expression,
    is_safe_from_code_point_arg, is_simple_inline_element, is_static_call, is_timer_call,
    is_undefined_constant, is_undefined_identifier, normalize_operator, nullish_check,
    numeric_literal_value, object_length_value, property_key_name, return_boolean,
    simple_regex_equivalent, single_expression_statement, single_spread_element,
    statement_contains_spread, static_member_callee, static_regexp_args,
};
use crate::scanner::Scanner;
use crate::{BanDependency, Diagnostic, DiagnosticData, DiagnosticFix};

impl<'a> Scanner<'a> {
    pub(crate) fn check_prefer_inline_equality(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "includes" || call.arguments.len() != 1 {
            return;
        }
        let Expression::ArrayExpression(array) = object.get_inner_expression() else {
            return;
        };
        if array.elements.is_empty() || array.elements.len() > 6 {
            return;
        }
        let value_text = self.text(call.arguments[0].span());
        let mut parts = Vec::new();
        for element in &array.elements {
            let Some(expression) = element.as_expression() else {
                return;
            };
            if !is_simple_inline_element(expression) {
                return;
            }
            parts.push(format!("{} === {value_text}", self.text(expression.span())));
        }
        self.report_with_fix(
            "prefer-inline-equality",
            "preferEquality",
            call.span,
            parts.join(" || "),
        );
    }

    pub(crate) fn check_prefer_string_from_char_code(&mut self, call: &'a CallExpression<'a>) {
        if !is_static_call(call, "String", "fromCodePoint") || call.arguments.is_empty() {
            return;
        }
        if !call.arguments.iter().all(is_safe_from_code_point_arg) {
            return;
        }
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        self.report_with_data(
            "prefer-string-fromcharcode",
            "preferFromCharCode",
            DiagnosticData::default(),
            member.property.span,
            Some(DiagnosticFix {
                start: member.property.span.start,
                end: member.property.span.end,
                replacement: CompactString::from("fromCharCode"),
            }),
        );
    }

    pub(crate) fn check_prefer_timer_args(&mut self, call: &'a CallExpression<'a>) {
        if !is_timer_call(call) || call.arguments.len() < 2 {
            return;
        }
        let Some(first_arg) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let delay_text = self.text(call.arguments[1].span());
        let timer_text = self.text(call.callee.span());
        let replacement = match first_arg.get_inner_expression() {
            Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
                if !arrow.expression {
                    return;
                };
                let Some(body) = expression_body(&arrow.body) else {
                    return;
                };
                let Expression::CallExpression(inner_call) = body.get_inner_expression() else {
                    return;
                };
                if matches!(
                    inner_call.callee.get_inner_expression(),
                    Expression::StaticMemberExpression(_)
                ) {
                    return;
                }
                let args = inner_call
                    .arguments
                    .iter()
                    .map(|argument| self.text(argument.span()).to_owned())
                    .collect::<Vec<_>>();
                format_timer_replacement(
                    &timer_text,
                    self.text(inner_call.callee.span()),
                    &delay_text,
                    &args,
                )
            }
            Expression::CallExpression(bind_call) if is_method_call(bind_call, "bind") => {
                let Some(bind_context) = bind_call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                else {
                    return;
                };
                if !is_null_or_undefined(bind_context) {
                    return;
                }
                let Some((fn_expression, _)) = static_member_callee(bind_call) else {
                    return;
                };
                let args = bind_call
                    .arguments
                    .iter()
                    .skip(1)
                    .map(|argument| self.text(argument.span()).to_owned())
                    .collect::<Vec<_>>();
                format_timer_replacement(
                    &timer_text,
                    self.text(fn_expression.span()),
                    &delay_text,
                    &args,
                )
            }
            _ => return,
        };
        self.report_with_fix("prefer-timer-args", "preferArgs", call.span, replacement);
    }
}
