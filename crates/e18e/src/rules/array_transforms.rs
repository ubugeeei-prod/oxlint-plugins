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
    pub(crate) fn check_prefer_exponentiation(&mut self, call: &'a CallExpression<'a>) {
        if !is_static_call(call, "Math", "pow") || call.arguments.len() != 2 {
            return;
        }
        let Some(base) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Some(exponent) = call.arguments.get(1).and_then(Argument::as_expression) else {
            return;
        };
        let replacement = format!(
            "({}) ** ({})",
            self.text(base.span()),
            self.text(exponent.span())
        );
        self.report_with_fix(
            "prefer-exponentiation-operator",
            "preferExponentiation",
            call.span,
            replacement,
        );
    }

    pub(crate) fn check_prefer_object_has_own(&mut self, call: &'a CallExpression<'a>) {
        if call.arguments.len() == 2
            && callee_path(&call.callee).as_deref() == Some("Object.prototype.hasOwnProperty.call")
        {
            let object = call.arguments[0].span();
            let property = call.arguments[1].span();
            let replacement = format!(
                "Object.hasOwn({}, {})",
                self.text(object),
                self.text(property)
            );
            self.report_with_fix(
                "prefer-object-has-own",
                "preferObjectHasOwn",
                call.span,
                replacement,
            );
            return;
        }

        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "hasOwnProperty" || call.arguments.len() != 1 {
            return;
        }
        let replacement = format!(
            "Object.hasOwn({}, {})",
            self.text(object.span()),
            self.text(call.arguments[0].span())
        );
        self.report_with_fix(
            "prefer-object-has-own",
            "preferObjectHasOwn",
            call.span,
            replacement,
        );
    }

    pub(crate) fn check_prefer_array_from_map(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "map" || call.arguments.len() != 1 {
            return;
        }
        let Expression::ArrayExpression(array) = object.get_inner_expression() else {
            return;
        };
        let Some(spread) = single_spread_element(array) else {
            return;
        };
        let iterable = self.text(spread.argument.span());
        let mapper = self.text(call.arguments[0].span());
        let replacement = format!("Array.from({iterable}, {mapper})");
        self.report_with_data(
            "prefer-array-from-map",
            "preferArrayFrom",
            DiagnosticData {
                iterable: Some(CompactString::from(iterable)),
                mapper: Some(CompactString::from(mapper)),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }

    pub(crate) fn check_prefer_array_fill(&mut self, call: &'a CallExpression<'a>) {
        if is_static_call(call, "Array", "from") && call.arguments.len() == 2 {
            let Some(Expression::ObjectExpression(object)) = call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .map(Expression::get_inner_expression)
            else {
                return;
            };
            let Some(length_value) = object_length_value(object) else {
                return;
            };
            let Some(callback) = call.arguments.get(1).and_then(Argument::as_expression) else {
                return;
            };
            let Some(value) = constant_callback_value(callback) else {
                return;
            };
            let length_text = self.text(length_value.span());
            let value_text = self.text(value.span());
            let replacement = format!("Array.from({{length: {length_text}}}).fill({value_text})");
            self.report_with_data(
                "prefer-array-fill",
                "preferFillArrayFrom",
                DiagnosticData {
                    length: Some(CompactString::from(length_text)),
                    value: Some(CompactString::from(value_text)),
                    ..DiagnosticData::default()
                },
                call.span,
                Some(DiagnosticFix {
                    start: call.span.start,
                    end: call.span.end,
                    replacement: CompactString::from(replacement),
                }),
            );
            return;
        }

        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        if property != "map" || call.arguments.len() != 1 {
            return;
        }
        let Expression::ArrayExpression(array) = object.get_inner_expression() else {
            return;
        };
        let Some(spread) = single_spread_element(array) else {
            return;
        };
        let Expression::CallExpression(array_call) = spread.argument.get_inner_expression() else {
            return;
        };
        if !matches!(array_call.callee.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Array")
            || array_call.arguments.len() != 1
        {
            return;
        }
        let Some(callback) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Some(value) = constant_callback_value(callback) else {
            return;
        };
        let length_text = self.text(array_call.arguments[0].span());
        let value_text = self.text(value.span());
        let replacement = format!("Array({length_text}).fill({value_text})");
        self.report_with_data(
            "prefer-array-fill",
            "preferFillSpreadMap",
            DiagnosticData {
                length: Some(CompactString::from(length_text)),
                value: Some(CompactString::from(value_text)),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }

    pub(crate) fn check_prefer_spread_syntax(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };

        if property == "concat"
            && !matches!(object.get_inner_expression(), Expression::Identifier(identifier) if identifier.name == "Buffer")
            && !call.arguments.is_empty()
        {
            let mut parts = SmallVec::<[CompactString; 8]>::new();
            if let Expression::ArrayExpression(array) = object.get_inner_expression() {
                for element in &array.elements {
                    if let Some(expression) = element.as_expression() {
                        parts.push(CompactString::from(self.text(expression.span())));
                    } else if let ArrayExpressionElement::SpreadElement(spread) = element {
                        parts.push(CompactString::from(self.text(spread.span)));
                    }
                }
            } else {
                parts.push(CompactString::from(format!(
                    "...{}",
                    self.text(object.span())
                )));
            }
            for argument in &call.arguments {
                if let Argument::SpreadElement(spread) = argument {
                    parts.push(CompactString::from(self.text(spread.span)));
                } else if let Some(Expression::ArrayExpression(array)) = argument
                    .as_expression()
                    .map(Expression::get_inner_expression)
                {
                    for element in &array.elements {
                        if let Some(expression) = element.as_expression() {
                            parts.push(CompactString::from(self.text(expression.span())));
                        }
                    }
                } else {
                    parts.push(CompactString::from(format!(
                        "...{}",
                        self.text(argument.span())
                    )));
                }
            }
            let replacement = format!("[{}]", parts.join(", "));
            self.report_with_fix(
                "prefer-spread-syntax",
                "preferSpreadArray",
                call.span,
                replacement,
            );
            return;
        }

        if is_static_call(call, "Array", "from") && call.arguments.len() == 1 {
            let Some(first_arg) = call.arguments.first() else {
                return;
            };
            if !matches!(first_arg, Argument::SpreadElement(_))
                && !matches!(
                    first_arg
                        .as_expression()
                        .map(Expression::get_inner_expression),
                    Some(Expression::ObjectExpression(_))
                )
            {
                let replacement = format!("[...{}]", self.text(first_arg.span()));
                self.report_with_fix(
                    "prefer-spread-syntax",
                    "preferSpreadArrayFrom",
                    call.span,
                    replacement,
                );
            }
            return;
        }

        if is_static_call(call, "Object", "assign") && call.arguments.len() >= 2 {
            let Some(Expression::ObjectExpression(first_object)) = call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .map(Expression::get_inner_expression)
            else {
                return;
            };
            if call
                .arguments
                .iter()
                .skip(1)
                .any(|arg| matches!(arg, Argument::SpreadElement(_)))
            {
                return;
            }
            let mut replacement = String::from("{");
            if !first_object.properties.is_empty() {
                let first_text = self.text(call.arguments[0].span());
                replacement.push_str(first_text.trim_start_matches('{').trim_end_matches('}'));
                replacement.push_str(", ");
            }
            let spreads: Vec<String> = call
                .arguments
                .iter()
                .skip(1)
                .map(|arg| format!("...{}", self.text(arg.span())))
                .collect();
            replacement.push_str(&spreads.join(", "));
            replacement.push('}');
            self.report_with_fix(
                "prefer-spread-syntax",
                "preferSpreadObject",
                call.span,
                replacement,
            );
            return;
        }

        if property == "apply" && call.arguments.len() == 2 {
            let Some(first_arg) = call.arguments.first().and_then(Argument::as_expression) else {
                return;
            };
            if !is_null_or_undefined(first_arg) {
                return;
            }
            let replacement = format!(
                "{}(...{})",
                self.text(object.span()),
                self.text(call.arguments[1].span())
            );
            self.report_with_fix(
                "prefer-spread-syntax",
                "preferSpreadFunction",
                call.span,
                replacement,
            );
        }
    }

    pub(crate) fn check_prefer_copy_method(&mut self, call: &'a CallExpression<'a>) {
        let Some((object, property)) = static_member_callee(call) else {
            return;
        };
        let Some((rule_name, message_id, method)) = (match property {
            "reverse" => Some(("prefer-array-to-reversed", "preferToReversed", "toReversed")),
            "sort" => Some(("prefer-array-to-sorted", "preferToSorted", "toSorted")),
            "splice" => Some(("prefer-array-to-spliced", "preferToSpliced", "toSpliced")),
            _ => None,
        }) else {
            return;
        };
        let Some(array) = copy_pattern_source(object) else {
            return;
        };
        let raw_text = self.text(array.span());
        let args = call
            .arguments
            .iter()
            .map(|argument| self.text(argument.span()).to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let access = if copy_pattern_optional(object) {
            "?."
        } else {
            "."
        };
        let replacement = format!("{raw_text}{access}{method}({args})");
        self.report_with_data(
            rule_name,
            message_id,
            DiagnosticData {
                array: Some(CompactString::from(raw_text)),
                ..DiagnosticData::default()
            },
            call.span,
            Some(DiagnosticFix {
                start: call.span.start,
                end: call.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }

    pub(crate) fn check_prefer_array_at(&mut self, member: &'a oxc_ast::ast::ComputedMemberExpression<'a>) {
        let Expression::BinaryExpression(binary) = member.expression.get_inner_expression() else {
            return;
        };
        if binary.operator != BinaryOperator::Subtraction || !is_number_literal(&binary.right, 1.0)
        {
            return;
        }
        let Expression::StaticMemberExpression(length_member) = binary.left.get_inner_expression()
        else {
            return;
        };
        if length_member.property.name != "length" {
            return;
        }
        let array_text = self.text(member.object.span());
        if array_text != self.text(length_member.object.span()) {
            return;
        }
        let replacement = format!("{array_text}.at(-1)");
        self.report_with_data(
            "prefer-array-at",
            "preferAt",
            DiagnosticData {
                array: Some(CompactString::from(array_text)),
                ..DiagnosticData::default()
            },
            member.span,
            Some(DiagnosticFix {
                start: member.span.start,
                end: member.span.end,
                replacement: CompactString::from(replacement),
            }),
        );
    }
}
