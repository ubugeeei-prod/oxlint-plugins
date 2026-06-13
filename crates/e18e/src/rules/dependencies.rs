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
    pub(crate) fn check_ban_dependency_import(&mut self, import: &'a ImportDeclaration<'a>) {
        self.check_ban_dependency_source(import.source.value.as_str(), import.source.span);
    }

    pub(crate) fn check_ban_dependency_require(&mut self, call: &'a CallExpression<'a>) {
        if !self.options.has_rule("ban-dependencies") {
            return;
        }
        if !matches!(call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "require")
        {
            return;
        }
        let Some(Expression::StringLiteral(source)) = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .map(Expression::get_inner_expression)
        else {
            return;
        };
        self.check_ban_dependency_source(source.value.as_str(), source.span);
    }

    pub(crate) fn check_ban_dependency_source(&mut self, source: &str, span: Span) {
        if !self.options.has_rule("ban-dependencies") {
            return;
        }
        for dependency in &self.options.banned_dependencies {
            if source == dependency.module_name
                || source
                    .strip_prefix(dependency.module_name.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
            {
                let diagnostic =
                    ban_dependency_diagnostic(dependency, span, self.source_text, &self.line_index);
                self.diagnostics.push(diagnostic);
                return;
            }
        }
    }
}
