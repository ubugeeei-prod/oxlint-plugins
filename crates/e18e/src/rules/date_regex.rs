//! Rule implementation methods, attached to [`Scanner`] via additional
//! `impl` blocks. The traversal in `scanner.rs` dispatches into these.

#![allow(
    unused_imports,
    reason = "Rule modules share a common import surface for readability; not every rule uses every helper."
)]

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, CallExpression, Expression, ImportDeclaration,
    NewExpression, ObjectPropertyKind, Statement,
};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{
    ban_dependency_diagnostic, binary_index_of_comparison, callee_path, constant_callback_value,
    copy_pattern_optional, copy_pattern_source, expression_body, expression_contains_spread,
    find_call_or_filter_length, find_or_filter_comparison, format_timer_replacement,
    function_body_contains_spread, includes_negation_for_constant, is_constant_expression,
    is_method_call, is_new_date_no_args, is_null_literal, is_null_or_undefined, is_number_literal,
    is_plain_regex_text, is_regex_expression, is_safe_from_code_point_arg,
    is_simple_inline_element, is_static_call, is_timer_call, is_undefined_constant,
    is_undefined_identifier, normalize_operator, nullish_check, numeric_literal_value,
    object_length_value, property_key_name, return_boolean, simple_regex_equivalent,
    single_expression_statement, single_spread_element, statement_contains_spread,
    static_member_callee, static_regexp_args, ExprContext, SomeSource,
};
use crate::scanner::Scanner;
use crate::{BanDependency, Diagnostic, DiagnosticData, DiagnosticFix};

impl<'a> Scanner<'a> {
    pub(crate) fn check_prefer_date_now_call(&mut self, call: &'a CallExpression<'a>) {
        if is_static_call(call, "Date", "now") {
            return;
        }
        if is_static_call(call, "Number", "") && call.arguments.len() == 1 {
            let Some(Expression::NewExpression(new_date)) = call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .map(Expression::get_inner_expression)
            else {
                return;
            };
            if is_new_date_no_args(new_date) {
                self.report_with_fix("prefer-date-now", "preferDateNow", call.span, "Date.now()");
            }
            return;
        }
        if !is_method_call(call, "getTime") || !call.arguments.is_empty() {
            return;
        }
        let Some((object, _)) = static_member_callee(call) else {
            return;
        };
        if let Expression::NewExpression(new_date) = object.get_inner_expression() {
            if is_new_date_no_args(new_date) {
                self.report_with_fix("prefer-date-now", "preferDateNow", call.span, "Date.now()");
            }
        }
    }

    pub(crate) fn check_prefer_date_now_new(&mut self, _new_expression: &'a NewExpression<'a>) {}

    pub(crate) fn check_prefer_date_now_unary(&mut self, unary: &'a oxc_ast::ast::UnaryExpression<'a>) {
        if unary.operator != UnaryOperator::UnaryPlus {
            return;
        }
        let Expression::NewExpression(new_date) = unary.argument.get_inner_expression() else {
            return;
        };
        if is_new_date_no_args(new_date) {
            self.report_with_fix("prefer-date-now", "preferDateNow", unary.span, "Date.now()");
        }
    }

    pub(crate) fn check_prefer_regex_test(&mut self, call: &'a CallExpression<'a>, context: ExprContext) {
        if context != ExprContext::Boolean {
            return;
        }
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if call.arguments.len() != 1 {
            return;
        }
        let (regex, string) =
            if property == "match" && is_regex_expression(call.arguments[0].as_expression()) {
                (call.arguments[0].span(), object.span())
            } else if property == "exec" && is_regex_expression(Some(object)) {
                (object.span(), call.arguments[0].span())
            } else {
                return;
            };
        let regex_text = self.text(regex);
        let string_text = self.text(string);
        self.report_with_data(
            "prefer-regex-test",
            "preferTest",
            DiagnosticData {
                regex: Some(CompactString::from(regex_text)),
                string: Some(CompactString::from(string_text)),
                original: Some(CompactString::from(self.text(call.span))),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(format!("{regex_text}.test({string_text})")),
            }),
        );
    }

    pub(crate) fn check_prefer_static_regex_call(&mut self, call: &'a CallExpression<'a>) {
        if self.function_depth == 0
            || !matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "RegExp")
        {
            return;
        }
        if static_regexp_args(&call.arguments) {
            self.report("prefer-static-regex", "preferStatic", call.span);
        }
    }

    pub(crate) fn check_prefer_static_regex_new(&mut self, new_expression: &'a NewExpression<'a>) {
        if self.function_depth == 0
            || !matches!(new_expression.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "RegExp")
        {
            return;
        }
        if static_regexp_args(&new_expression.arguments) {
            self.report("prefer-static-regex", "preferStatic", new_expression.span);
        }
    }
}
