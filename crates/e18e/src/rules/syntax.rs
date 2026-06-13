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
    pub(crate) fn check_prefer_url_canparse(
        &mut self,
        statement: &'a oxc_ast::ast::TryStatement<'a>,
    ) {
        let Some(handler) = &statement.handler else {
            return;
        };
        if statement.block.body.len() != 2 || handler.body.body.len() != 1 {
            return;
        }
        let Statement::ExpressionStatement(first) = &statement.block.body[0] else {
            return;
        };
        let Expression::NewExpression(new_url) = first.expression.get_inner_expression() else {
            return;
        };
        if !matches!(new_url.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "URL")
            || new_url.arguments.is_empty()
        {
            return;
        }
        let Statement::ReturnStatement(ok_return) = &statement.block.body[1] else {
            return;
        };
        let Statement::ReturnStatement(error_return) = &handler.body.body[0] else {
            return;
        };
        if !return_boolean(ok_return, true) || !return_boolean(error_return, false) {
            return;
        }
        let args = new_url
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        self.report_with_fix(
            "prefer-url-canparse",
            "preferCanParse",
            statement.span,
            format!("return URL.canParse({args})"),
        );
    }

    pub(crate) fn check_no_delete_property(
        &mut self,
        unary: &'a oxc_ast::ast::UnaryExpression<'a>,
        context: ExprContext,
    ) {
        if unary.operator != UnaryOperator::Delete {
            return;
        }
        let member_span = match unary.argument.get_inner_expression() {
            Expression::StaticMemberExpression(member) => Some(member.span),
            Expression::ComputedMemberExpression(member)
                if matches!(
                    member.expression.get_inner_expression(),
                    Expression::StringLiteral(_)
                ) =>
            {
                Some(member.span)
            }
            _ => None,
        };
        let Some(member_span) = member_span else {
            return;
        };
        let fix = if context == ExprContext::Statement {
            Some(DiagnosticFix {
                start: unary.span.start,
                end: unary.span.end,
                replacement: CompactString::from(format!("{} = undefined", self.text(member_span))),
            })
        } else {
            None
        };
        self.report_with_data(
            "no-delete-property",
            "noDeleteProperty",
            DiagnosticData::default(),
            unary.span,
            fix,
        );
    }
}
