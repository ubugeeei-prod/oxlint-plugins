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
    pub(crate) fn check_prefer_includes_binary(&mut self, binary: &'a oxc_ast::ast::BinaryExpression<'a>) {
        let Some((index_call, constant, reversed)) =
            binary_index_of_comparison(&binary.left, &binary.right)
        else {
            return;
        };
        let op = normalize_operator(binary.operator, reversed);
        let Some(should_negate) = includes_negation_for_constant(op, constant) else {
            return;
        };
        self.report_index_of_as_includes(binary.span, index_call, should_negate);
    }

    pub(crate) fn check_prefer_includes_unary(&mut self, unary: &'a oxc_ast::ast::UnaryExpression<'a>) {
        if unary.operator == UnaryOperator::BitwiseNot {
            if let Expression::CallExpression(call) = unary.argument.get_inner_expression() {
                if is_method_call(call, "indexOf") {
                    self.report_index_of_as_includes(unary.span, call, false);
                }
            }
        } else if unary.operator == UnaryOperator::LogicalNot {
            let Expression::UnaryExpression(inner) = unary.argument.get_inner_expression() else {
                return;
            };
            if inner.operator == UnaryOperator::BitwiseNot {
                if let Expression::CallExpression(call) = inner.argument.get_inner_expression() {
                    if is_method_call(call, "indexOf") {
                        self.report_index_of_as_includes(unary.span, call, true);
                    }
                }
            }
        }
    }

    pub(crate) fn report_index_of_as_includes(
        &mut self,
        span: Span,
        index_call: &'a CallExpression<'a>,
        should_negate: bool,
    ) {
        let Some((object, _)) = static_member_callee(index_call) else {
            return;
        };
        let args = index_call
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let replacement = if should_negate {
            format!("!{}.includes({args})", self.text(object.span()))
        } else {
            format!("{}.includes({args})", self.text(object.span()))
        };
        self.report_with_fix("prefer-includes", "preferIncludes", span, replacement);
    }

    pub(crate) fn check_no_indexof_equality(&mut self, binary: &'a oxc_ast::ast::BinaryExpression<'a>) {
        if !matches!(
            binary.operator,
            BinaryOperator::Equality | BinaryOperator::StrictEquality
        ) {
            return;
        }
        let Some((index_call, constant, _)) =
            binary_index_of_comparison(&binary.left, &binary.right)
        else {
            return;
        };
        let Some(index) = numeric_literal_value(constant) else {
            return;
        };
        if index < 0.0 || index.fract() != 0.0 {
            return;
        }
        let Some((object, _)) = static_member_callee(index_call) else {
            return;
        };
        let Some(search) = index_call.arguments.first() else {
            return;
        };
        let object_text = self.text(object.span());
        let search_text = self.text(search.span());
        if index == 0.0 {
            self.report_with_fix(
                "no-indexof-equality",
                "preferStartsWith",
                binary.span,
                format!("{object_text}.startsWith({search_text})"),
            );
        } else {
            let index_text = format!("{index:.0}");
            self.report_with_data(
                "no-indexof-equality",
                "preferDirectAccess",
                DiagnosticData {
                    array: Some(CompactString::from(object_text)),
                    item: Some(CompactString::from(search_text)),
                    index: Some(CompactString::from(index_text.clone())),
                    ..DiagnosticData::default()
                },
                binary.span,
                Some(DiagnosticFix {
                    start: binary.span.start,
                    end: binary.span.end,
                    replacement: CompactString::from(format!(
                        "{object_text}[{index_text}] === {search_text}"
                    )),
                }),
            );
        }
    }

    pub(crate) fn check_prefer_nullish_conditional(
        &mut self,
        conditional: &'a oxc_ast::ast::ConditionalExpression<'a>,
    ) {
        let Some(check) = nullish_check(self.source_text, &conditional.test) else {
            return;
        };
        let compare = if check.checks_for_nullish {
            &conditional.alternate
        } else {
            &conditional.consequent
        };
        let default = if check.checks_for_nullish {
            &conditional.consequent
        } else {
            &conditional.alternate
        };
        if self.text(check.value.span()) != self.text(compare.span()) {
            return;
        }
        let replacement = format!(
            "{} ?? {}",
            self.text(check.value.span()),
            self.text(default.span())
        );
        self.report_with_fix(
            "prefer-nullish-coalescing",
            "preferNullishCoalescing",
            conditional.span,
            replacement,
        );
    }

    pub(crate) fn check_prefer_nullish_assignment(&mut self, statement: &'a oxc_ast::ast::IfStatement<'a>) {
        if statement.alternate.is_some() {
            return;
        }
        let Some(check) = nullish_check(self.source_text, &statement.test) else {
            return;
        };
        if !check.checks_for_nullish {
            return;
        }
        let Some(expression_statement) = single_expression_statement(&statement.consequent) else {
            return;
        };
        let Expression::AssignmentExpression(assignment) =
            expression_statement.expression.get_inner_expression()
        else {
            return;
        };
        if assignment.operator != AssignmentOperator::Assign {
            return;
        }
        if self.text(check.value.span()) != self.text(assignment.left.span()) {
            return;
        }
        let replacement = format!(
            "{} ??= {}",
            self.text(assignment.left.span()),
            self.text(assignment.right.span())
        );
        self.report_with_fix(
            "prefer-nullish-coalescing",
            "preferNullishCoalescingAssignment",
            statement.span,
            replacement,
        );
    }
}
