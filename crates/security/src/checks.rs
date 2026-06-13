//! Per-rule diagnostic checks for the security scanner.

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{
    BUFFER_READ_METHODS, BUFFER_WRITE_METHODS, argument_is_literal, contains_timing_keyword,
    expression_type, fs_argument_indices, is_dangerous_bidi, is_unsafe_regex, join_usize,
    source_line_at, static_member_property, string_literal_value,
};
use crate::scanner::Scanner;
use crate::{CHILD_PROCESS_PACKAGES, DiagnosticData, FS_PACKAGES, ParentKind};

impl<'a> Scanner<'a> {
    pub(crate) fn check_call_expression(
        &mut self,
        call: &'a CallExpression<'a>,
        parent: ParentKind,
    ) {
        if let Some(package_name) = self.require_package_name(call)
            && CHILD_PROCESS_PACKAGES.contains(&package_name)
            && !matches!(
                parent,
                ParentKind::VariableInit | ParentKind::AssignmentRight | ParentKind::MemberObject
            )
        {
            self.report_with_data(
                "detect-child-process",
                "require",
                DiagnosticData {
                    value: Some(CompactString::from(package_name)),
                    ..DiagnosticData::default()
                },
                call.span,
            );
        }

        if call.callee.is_specific_id("eval")
            && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
            && !argument.is_literal()
        {
            self.report_with_data(
                "detect-eval-with-expression",
                "nonLiteral",
                DiagnosticData {
                    argument_type: Some(CompactString::from(expression_type(argument))),
                    ..DiagnosticData::default()
                },
                call.span,
            );
        }

        if call.callee.is_specific_id("require")
            && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
            && !self.is_static_expression(argument, 0)
        {
            self.report("detect-non-literal-require", "nonLiteral", call.span);
        }

        if let Some(path) = self.import_access_path(&call.callee, &CHILD_PROCESS_PACKAGES)
            && path.path.len() == 1
            && path.path[0].as_str() == "exec"
            && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
            && !self.is_static_expression(argument, 0)
        {
            self.report("detect-child-process", "execNonLiteral", call.span);
        }

        self.check_buffer_noassert(call);
        self.check_no_csrf_before_method_override(call);
        self.check_non_literal_fs_filename(call);
    }

    pub(crate) fn check_new_expression(&mut self, new_expression: &'a NewExpression<'a>) {
        if new_expression.callee.is_specific_id("Buffer")
            && let Some(argument) = new_expression
                .arguments
                .first()
                .and_then(Argument::as_expression)
            && !argument.is_literal()
        {
            self.report("detect-new-buffer", "found", new_expression.span);
        }

        if new_expression.callee.is_specific_id("RegExp")
            && let Some(argument) = new_expression
                .arguments
                .first()
                .and_then(Argument::as_expression)
        {
            if !self.is_static_expression(argument, 0) {
                self.report(
                    "detect-non-literal-regexp",
                    "nonLiteral",
                    new_expression.span,
                );
            } else if let Some(pattern) = string_literal_value(argument)
                && is_unsafe_regex(pattern)
            {
                self.report("detect-unsafe-regex", "newRegExp", new_expression.span);
            }
        }
    }

    fn check_buffer_noassert(&mut self, call: &'a CallExpression<'a>) {
        let Some(method) = static_member_property(&call.callee) else {
            return;
        };
        let index = if BUFFER_READ_METHODS.contains(&method) {
            Some(1)
        } else if BUFFER_WRITE_METHODS.contains(&method) {
            Some(2)
        } else {
            None
        };

        if let Some(index) = index
            && let Some(argument) = call.arguments.get(index).and_then(Argument::as_expression)
            && matches!(argument.get_inner_expression(), Expression::BooleanLiteral(value) if value.value)
        {
            self.report_with_data(
                "detect-buffer-noassert",
                "found",
                DiagnosticData {
                    method: Some(CompactString::from(method)),
                    ..DiagnosticData::default()
                },
                call.callee.span(),
            );
        }
    }

    fn check_no_csrf_before_method_override(&mut self, call: &'a CallExpression<'a>) {
        if !call.callee.is_specific_member_access("express", "csrf")
            && !call
                .callee
                .is_specific_member_access("express", "methodOverride")
        {
            return;
        }

        if call
            .callee
            .is_specific_member_access("express", "methodOverride")
            && self.csrf_seen
        {
            self.report("detect-no-csrf-before-method-override", "found", call.span);
        }
        if call.callee.is_specific_member_access("express", "csrf") {
            self.csrf_seen = true;
        }
    }

    pub(crate) fn check_disable_mustache_escape(
        &mut self,
        span: Span,
        left: &'a AssignmentTarget<'a>,
        right: &'a Expression<'a>,
    ) {
        if !matches!(left, AssignmentTarget::StaticMemberExpression(member) if member.property.name == "escapeMarkup")
        {
            return;
        }
        if matches!(right.get_inner_expression(), Expression::BooleanLiteral(value) if !value.value)
        {
            self.report("detect-disable-mustache-escape", "found", span);
        }
    }

    fn check_non_literal_fs_filename(&mut self, call: &'a CallExpression<'a>) {
        if call.callee.is_specific_id("require") || call.arguments.iter().all(argument_is_literal) {
            return;
        }

        let Some(path) = self.import_access_path(&call.callee, &FS_PACKAGES) else {
            return;
        };
        let fn_name = match path.path.as_slice() {
            [name] => name.as_str(),
            [_, name] => name.as_str(),
            _ => return,
        };
        let Some(indices_to_check) = fs_argument_indices(fn_name) else {
            return;
        };

        let mut indices: SmallVec<[usize; 2]> = SmallVec::new();
        for index in indices_to_check {
            if let Some(argument) = call.arguments.get(*index).and_then(Argument::as_expression)
                && !self.is_static_expression(argument, 0)
            {
                indices.push(*index);
            }
        }

        if !indices.is_empty() {
            let joined = join_usize(&indices);
            self.report_with_data(
                "detect-non-literal-fs-filename",
                "nonLiteral",
                DiagnosticData {
                    fn_name: Some(CompactString::from(fn_name)),
                    package_name: Some(path.package_name),
                    indices: Some(joined),
                    ..DiagnosticData::default()
                },
                call.span,
            );
        }
    }

    pub(crate) fn check_object_injection(
        &mut self,
        span: Span,
        property: &'a Expression<'a>,
        parent: ParentKind,
    ) {
        if !matches!(property.get_inner_expression(), Expression::Identifier(_)) {
            return;
        }
        let message_id = match parent {
            ParentKind::VariableInit => "variable",
            ParentKind::CallCallee => "functionCall",
            _ => "generic",
        };
        self.report("detect-object-injection", message_id, span);
    }

    pub(crate) fn check_possible_timing_attack(&mut self, span: Span, test: &'a Expression<'a>) {
        let Expression::BinaryExpression(binary) = test.get_inner_expression() else {
            return;
        };
        if !matches!(
            binary.operator,
            BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
        ) {
            return;
        }
        if contains_timing_keyword(&binary.left) {
            self.report_with_data(
                "detect-possible-timing-attacks",
                "found",
                DiagnosticData {
                    side: Some(CompactString::from("left")),
                    ..DiagnosticData::default()
                },
                span,
            );
        } else if contains_timing_keyword(&binary.right) {
            self.report_with_data(
                "detect-possible-timing-attacks",
                "found",
                DiagnosticData {
                    side: Some(CompactString::from("right")),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
    }

    pub(crate) fn scan_bidi_characters(&mut self) {
        for (start, ch) in self.source_text.char_indices() {
            if !is_dangerous_bidi(ch) {
                continue;
            }
            let end = start + ch.len_utf8();
            let in_comment = self
                .comment_spans
                .iter()
                .any(|span| span.start as usize <= start && end <= span.end as usize);
            let line_text = source_line_at(self.source_text, start);
            self.report_with_data(
                "detect-bidi-characters",
                if in_comment { "comment" } else { "code" },
                DiagnosticData {
                    text: Some(CompactString::from(line_text)),
                    ..DiagnosticData::default()
                },
                Span::new(start as u32, end as u32),
            );
        }
    }
}
