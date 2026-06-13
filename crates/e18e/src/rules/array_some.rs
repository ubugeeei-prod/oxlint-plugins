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
    pub(crate) fn check_prefer_array_some_binary(&mut self, binary: &'a oxc_ast::ast::BinaryExpression<'a>) {
        let Some((kind, constant, reversed)) =
            find_or_filter_comparison(&binary.left, &binary.right)
        else {
            return;
        };
        let op = normalize_operator(binary.operator, reversed);
        let should_negate = match kind {
            SomeSource::Find(call) if is_undefined_constant(constant) => match op {
                BinaryOperator::StrictEquality | BinaryOperator::Equality => Some(true),
                BinaryOperator::StrictInequality | BinaryOperator::Inequality => Some(false),
                _ => None,
            }
            .map(|negate| (call, negate)),
            SomeSource::FilterLength(call) => {
                let Some(value) = numeric_literal_value(constant) else {
                    return;
                };
                let negate = if value == 0.0 {
                    match op {
                        BinaryOperator::StrictEquality
                        | BinaryOperator::Equality
                        | BinaryOperator::LessEqualThan => Some(true),
                        BinaryOperator::StrictInequality
                        | BinaryOperator::Inequality
                        | BinaryOperator::GreaterThan => Some(false),
                        _ => None,
                    }
                } else if value == 1.0 {
                    match op {
                        BinaryOperator::LessThan => Some(true),
                        BinaryOperator::GreaterEqualThan => Some(false),
                        _ => None,
                    }
                } else {
                    None
                };
                let Some(negate) = negate else {
                    return;
                };
                Some((call, negate))
            }
            _ => None,
        };
        if let Some((call, negate)) = should_negate {
            self.report_array_some(binary.span, call, negate);
        }
    }

    pub(crate) fn check_prefer_array_some_unary(&mut self, unary: &'a oxc_ast::ast::UnaryExpression<'a>) {
        if unary.operator != UnaryOperator::LogicalNot {
            return;
        }
        if let Some(call) = find_call_or_filter_length(&unary.argument) {
            self.report_array_some(unary.span, call, true);
            return;
        }
        let Expression::UnaryExpression(inner) = unary.argument.get_inner_expression() else {
            return;
        };
        if inner.operator == UnaryOperator::LogicalNot {
            if let Some(call) = find_call_or_filter_length(&inner.argument) {
                self.report_array_some(unary.span, call, false);
            }
        }
    }

    pub(crate) fn check_prefer_array_some_call(&mut self, call: &'a CallExpression<'a>, context: ExprContext) {
        if context == ExprContext::Boolean && is_method_call(call, "find") {
            self.report_array_some(call.span, call, false);
        }
    }

    pub(crate) fn check_filter_length_member(
        &mut self,
        member: &'a oxc_ast::ast::StaticMemberExpression<'a>,
        context: ExprContext,
    ) {
        if context == ExprContext::Boolean && member.property.name == "length" {
            if let Expression::CallExpression(call) = member.object.get_inner_expression() {
                if is_method_call(call, "filter") {
                    self.report_array_some(member.span, call, false);
                }
            }
        }
    }

    pub(crate) fn report_array_some(&mut self, span: Span, call: &'a CallExpression<'a>, negate: bool) {
        let Some((object, _)) = static_member_callee(call) else {
            return;
        };
        let args = call
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let replacement = if negate {
            format!("!{}.some({args})", self.text(object.span()))
        } else {
            format!("{}.some({args})", self.text(object.span()))
        };
        self.report_with_fix("prefer-array-some", "preferArraySome", span, replacement);
    }
}
