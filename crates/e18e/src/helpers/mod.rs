//! Helper functions shared across the rule modules and the AST traversal
//! scanner. Items here keep a stable `crate::helpers::*` surface; the actual
//! implementations live in the submodules below.

mod ast;
mod comparisons;
mod constants;
mod literals;

use oxc_ast::ast::Expression;
use oxc_ast::ast::CallExpression;
use oxc_span::Span;

use crate::{BanDependency, Diagnostic, DiagnosticData, LineIndex};

pub(crate) use ast::{
    callee_path, expression_body, is_method_call, is_static_call, is_timer_call,
    object_length_value, property_key_name, single_expression_statement, single_spread_element,
    static_member_callee,
};
pub(crate) use comparisons::{
    binary_index_of_comparison, find_call_or_filter_length, find_or_filter_comparison,
    includes_negation_for_constant, normalize_operator, nullish_check,
};
pub(crate) use constants::{
    constant_callback_value, copy_pattern_optional, copy_pattern_source,
    expression_contains_spread, function_body_contains_spread, is_constant_expression,
    statement_contains_spread,
};
pub(crate) use literals::{
    format_timer_replacement, is_new_date_no_args, is_null_literal, is_null_or_undefined,
    is_number_literal, is_plain_regex_text, is_regex_expression, is_safe_from_code_point_arg,
    is_simple_inline_element, is_undefined_constant, is_undefined_identifier,
    numeric_literal_value, return_boolean, simple_regex_equivalent, static_regexp_args,
};

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
