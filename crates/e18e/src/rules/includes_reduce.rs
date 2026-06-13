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
    pub(crate) fn check_prefer_includes_over_regex_test(&mut self, call: &'a CallExpression<'a>) {
        if !is_method_call(call, "test") || call.arguments.len() != 1 {
            return;
        }
        let Some((regex, _)) = static_member_callee(call) else {
            return;
        };
        let Some((message_id, replacement_method, value)) =
            simple_regex_equivalent(self.text(regex.span()))
        else {
            return;
        };
        let string_text = self.text(call.arguments[0].span());
        let replacement = if replacement_method == "===" {
            format!("{string_text} === {value:?}")
        } else {
            format!("{string_text}.{replacement_method}({value:?})")
        };
        self.report_with_fix(
            "prefer-includes-over-regex-test",
            message_id,
            call.span,
            replacement,
        );
    }

    pub(crate) fn check_no_spread_in_reduce(&mut self, call: &'a CallExpression<'a>) {
        let Some((_, property)) = static_member_callee(call) else {
            return;
        };
        if property != "reduce" {
            return;
        }
        for argument in &call.arguments {
            let Some(expression) = argument.as_expression() else {
                continue;
            };
            if function_body_contains_spread(expression) {
                self.report("no-spread-in-reduce", "noSpreadInReduce", expression.span());
            }
        }
    }

    pub(crate) fn check_prefer_static_collator(&mut self, call: &'a CallExpression<'a>) {
        if self.function_depth == 0 {
            return;
        }
        if is_static_call(call, "Intl", "Collator") {
            self.report("prefer-static-collator", "preferStaticCollator", call.span);
        }
    }
}
