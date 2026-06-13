//! Regexp-specific diagnostic checks: literals, constructors, flags, and
//! pattern-level rules. These methods are reached through the AST traversal in
//! `traversal.rs` and rely on helpers in `helpers.rs` / `pattern.rs`.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, CallExpression, Expression, NewExpression, RegExpLiteral, StaticMemberExpression,
};
use oxc_regular_expression::{ConstructorParser, Options as RegExpOptions};
use oxc_span::Span;
use oxlint_plugins_carton::CompactString;

use crate::helpers::{
    duplicate_flag, first_control_character, first_fixed_unicode_escape, first_hex_x_escape,
    first_invisible_character, first_non_standard_flag, first_octal_escape,
    first_surrogate_pair_escape, first_uppercase_hex_escape, mention_char,
    pattern_has_empty_string_literal, sorted_flags, string_literal_value_with_span,
};
use crate::pattern::PatternAnalysis;
use crate::scanner::Scanner;
use crate::types::DiagnosticData;

/// Returns `true` when `replacement` contains a `$N` backreference (where `N`
/// is a single ASCII digit 1-9). Used by `prefer-named-replacement` to decide
/// whether a string replacement is using numbered backreferences. `$$` (escaped
/// dollar) and `$&` (whole match) are intentionally skipped.
fn contains_numeric_backreference(replacement: &str) -> bool {
    let bytes = replacement.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'$' {
            let next = bytes[index + 1];
            if next == b'$' {
                index += 2;
                continue;
            }
            if next.is_ascii_digit() && next != b'0' {
                return true;
            }
        }
        index += 1;
    }
    false
}

/// Static `RegExp.*` properties that upstream `eslint-plugin-regexp/no-legacy-features`
/// reports. Excludes special-character aliases such as `$&`, `$+`, `` $` ``, and
/// `$'`, which cannot be accessed through plain static member syntax (those go
/// through computed-member access and are not handled by this rule here).
static LEGACY_REGEXP_STATIC_PROPERTIES: &[&str] = &[
    "$1",
    "$2",
    "$3",
    "$4",
    "$5",
    "$6",
    "$7",
    "$8",
    "$9",
    "input",
    "$_",
    "lastMatch",
    "lastParen",
    "leftContext",
    "rightContext",
];

impl<'a> Scanner<'a> {
    pub(crate) fn check_static_member_expression(
        &mut self,
        member: &'a StaticMemberExpression<'a>,
    ) {
        let Expression::Identifier(identifier) = &member.object else {
            return;
        };
        if identifier.name != "RegExp" {
            return;
        }
        let property = member.property.name.as_str();
        if LEGACY_REGEXP_STATIC_PROPERTIES.contains(&property) {
            self.report_with_data(
                "no-legacy-features",
                "staticProperty",
                DiagnosticData {
                    expr: Some(CompactString::from(property)),
                    ..DiagnosticData::default()
                },
                member.span,
            );
        }
    }

    pub(crate) fn check_call_expression(&mut self, call: &'a CallExpression<'a>) {
        if call.callee.is_specific_id("RegExp") {
            self.check_regexp_constructor(call.span, &call.arguments);
        }
        self.check_prefer_regexp_exec(call);
        self.check_no_missing_g_flag(call);
        self.check_prefer_named_replacement(call);
    }

    /// `prefer-named-replacement`: when `<expr>.replace(<regexp with named
    /// captures>, "...$N...")` mixes numbered backreferences with a regex that
    /// has at least one named capture, the named form `$<name>` is clearer.
    /// We only attempt the literal-regexp + literal-replacement case; dynamic
    /// arguments are deferred.
    fn check_prefer_named_replacement(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "replace" && method != "replaceAll" {
            return;
        }
        if call.arguments.len() < 2 {
            return;
        }
        let Some(arg0) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Expression::RegExpLiteral(literal) = arg0.get_inner_expression() else {
            return;
        };
        if !literal.regex.pattern.text.as_str().contains("(?<") {
            return;
        }
        let Some(arg1) = call.arguments.get(1).and_then(Argument::as_expression) else {
            return;
        };
        let Expression::StringLiteral(replacement) = arg1.get_inner_expression() else {
            return;
        };
        if !contains_numeric_backreference(replacement.value.as_str()) {
            return;
        }
        self.report("prefer-named-replacement", "unexpected", call.span);
    }

    /// `no-missing-g-flag`: `<expr>.matchAll(<regexp without 'g'>)` and
    /// `<expr>.replaceAll(<regexp without 'g'>, ...)` throw at runtime (the
    /// engine requires the global flag). Flag the literal-regexp case
    /// statically; constructor calls are deferred for the same type-info
    /// reason as `prefer-regexp-exec`.
    fn check_no_missing_g_flag(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "matchAll" && method != "replaceAll" {
            return;
        }
        if call.arguments.is_empty() {
            return;
        }
        let Some(argument) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Expression::RegExpLiteral(literal) = argument.get_inner_expression() else {
            return;
        };
        let flags = literal
            .raw
            .as_ref()
            .and_then(|raw| raw.as_str().rsplit_once('/').map(|(_, flags)| flags))
            .unwrap_or("");
        if flags.contains('g') {
            return;
        }
        let mut method_text = CompactString::new("");
        method_text.push_str(method);
        self.report_with_data(
            "no-missing-g-flag",
            "unexpected",
            DiagnosticData {
                expr: Some(method_text),
                ..DiagnosticData::default()
            },
            call.span,
        );
    }

    /// `prefer-regexp-exec`: flag `<expr>.match(<regexp literal without 'g'>)`
    /// and recommend `<regexp literal>.exec(<expr>)`. We can only act on
    /// RegExp literals (constructor calls need type information to identify
    /// the receiver as a string). Patterns without a `g` flag are the
    /// canonical case the rule targets.
    fn check_prefer_regexp_exec(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        if member.property.name != "match" {
            return;
        }
        if call.arguments.len() != 1 {
            return;
        }
        let Some(argument) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Expression::RegExpLiteral(literal) = argument.get_inner_expression() else {
            return;
        };
        let flags = literal
            .raw
            .as_ref()
            .and_then(|raw| raw.as_str().rsplit_once('/').map(|(_, flags)| flags))
            .unwrap_or("");
        if flags.contains('g') {
            return;
        }
        self.report("prefer-regexp-exec", "unexpected", call.span);
    }

    pub(crate) fn check_new_expression(&mut self, new_expression: &'a NewExpression<'a>) {
        if new_expression.callee.is_specific_id("RegExp") {
            self.check_regexp_constructor(new_expression.span, &new_expression.arguments);
        }
    }

    pub(crate) fn check_regexp_literal(&mut self, literal: &'a RegExpLiteral<'a>) {
        let pattern = literal.regex.pattern.text.as_str();
        let flags = literal
            .raw
            .as_ref()
            .and_then(|raw| raw.as_str().rsplit_once('/').map(|(_, flags)| flags))
            .unwrap_or("");
        self.check_regexp(pattern, flags, literal.span, false, None, None);
    }

    fn check_regexp_constructor(
        &mut self,
        span: Span,
        arguments: &'a oxc_allocator::Vec<'a, Argument<'a>>,
    ) {
        let Some(pattern_argument) = arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Some((pattern, pattern_span)) = string_literal_value_with_span(pattern_argument) else {
            return;
        };
        let flags = arguments
            .get(1)
            .and_then(Argument::as_expression)
            .and_then(string_literal_value_with_span);
        let flags_value = flags.map_or("", |(value, _)| value);
        self.check_regexp(
            pattern,
            flags_value,
            span,
            true,
            Some(pattern_span),
            flags.map(|(_, span)| span),
        );
    }

    fn check_regexp(
        &mut self,
        pattern: &str,
        flags: &str,
        span: Span,
        is_constructor: bool,
        pattern_span: Option<Span>,
        flags_span: Option<Span>,
    ) {
        if let Some(flag) = duplicate_flag(flags) {
            self.report_with_data(
                "no-invalid-regexp",
                "duplicateFlag",
                DiagnosticData {
                    flag: Some(CompactString::from(flag)),
                    ..DiagnosticData::default()
                },
                span,
            );
            return;
        }
        if flags.contains('u') && flags.contains('v') {
            self.report("no-invalid-regexp", "uvFlag", span);
            return;
        }
        if let Some(flag) = first_non_standard_flag(flags) {
            // Reported alongside any constructor parse error below; this rule
            // exists as its own diagnostic so users can target it independently
            // of `no-invalid-regexp`. We intentionally do not early-return.
            let mut flag_text = CompactString::new("");
            flag_text.push(flag);
            self.report_with_data(
                "no-non-standard-flag",
                "unexpected",
                DiagnosticData {
                    flag: Some(flag_text),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let (true, Some(message)) = (
            is_constructor,
            self.constructor_parse_error(pattern_span, flags_span),
        ) {
            self.report_with_data(
                "no-invalid-regexp",
                "error",
                DiagnosticData {
                    message: Some(message),
                    ..DiagnosticData::default()
                },
                span,
            );
            return;
        }

        self.check_flag_style(flags, span);
        self.check_pattern_rules(pattern, span);
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "Oxc regexp parser exposes display text; this allocation is only diagnostic data."
    )]
    fn constructor_parse_error(
        &self,
        pattern_span: Option<Span>,
        flags_span: Option<Span>,
    ) -> Option<CompactString> {
        let pattern_span = pattern_span?;
        let allocator = Allocator::default();
        let parsed = ConstructorParser::new(
            &allocator,
            pattern_span.source_text(self.source_text),
            flags_span.map(|span| span.source_text(self.source_text)),
            RegExpOptions {
                pattern_span_offset: pattern_span.start,
                flags_span_offset: flags_span.map_or(0, |span| span.start),
            },
        )
        .parse();
        match parsed {
            Ok(_) => None,
            Err(error) => Some(CompactString::from(error.to_string().as_str())),
        }
    }

    fn check_flag_style(&mut self, flags: &str, span: Span) {
        let sorted_flags = sorted_flags(flags);
        if flags != sorted_flags.as_str() {
            self.report_with_data(
                "sort-flags",
                "sortFlags",
                DiagnosticData {
                    flags: Some(CompactString::from(flags)),
                    sorted_flags: Some(sorted_flags),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if !flags.contains('u') && !flags.contains('v') {
            self.report("require-unicode-regexp", "require", span);
        }
        if !flags.contains('v') {
            self.report("require-unicode-sets-regexp", "require", span);
        }
    }

    fn check_pattern_rules(&mut self, pattern: &str, span: Span) {
        let mut analysis = PatternAnalysis::new();
        analysis.scan(pattern);

        if analysis.has_empty_character_class {
            self.report("no-empty-character-class", "empty", span);
        }
        if analysis.has_empty_group {
            self.report("no-empty-group", "unexpected", span);
        }
        if analysis.has_empty_capturing_group {
            self.report("no-empty-capturing-group", "unexpected", span);
        }
        if analysis.has_empty_alternative {
            self.report("no-empty-alternative", "empty", span);
        }
        if analysis.has_zero_quantifier {
            self.report("no-zero-quantifier", "unexpected", span);
        }
        if let Some(escape) = first_octal_escape(pattern) {
            self.report_with_data(
                "no-octal",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(escape)),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(ch) = first_control_character(pattern) {
            self.report_with_data(
                "no-control-character",
                "unexpected",
                DiagnosticData {
                    char_text: Some(mention_char(ch)),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if analysis.has_escape_backspace_in_class {
            self.report("no-escape-backspace", "unexpected", span);
        }
        if let Some(expr) = analysis.first_plus_quantifier {
            self.report_with_data(
                "prefer-plus-quantifier",
                "unexpected",
                DiagnosticData {
                    expr: Some(expr),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(expr) = analysis.first_star_quantifier {
            self.report_with_data(
                "prefer-star-quantifier",
                "unexpected",
                DiagnosticData {
                    expr: Some(expr),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(expr) = analysis.first_question_quantifier {
            self.report_with_data(
                "prefer-question-quantifier",
                "unexpected",
                DiagnosticData {
                    expr: Some(expr),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some((expr, replacement)) = analysis.first_useless_two_nums_quantifier {
            self.report_with_data(
                "no-useless-two-nums-quantifier",
                "unexpected",
                DiagnosticData {
                    expr: Some(expr),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if analysis.has_unnamed_capturing_group {
            self.report("prefer-named-capture-group", "required", span);
        }
        if analysis.has_match_any_class {
            self.report("match-any", "unexpected", span);
        }
        if let Some(negated) = analysis.first_digit_class {
            self.report_with_data(
                "prefer-d",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(if negated {
                        "[^0-9]"
                    } else {
                        "[0-9]"
                    })),
                    replacement: Some(CompactString::from(if negated { "\\D" } else { "\\d" })),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(negated) = analysis.first_word_class {
            self.report_with_data(
                "prefer-w",
                "unexpected",
                DiagnosticData {
                    replacement: Some(CompactString::from(if negated { "\\W" } else { "\\w" })),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(ch) = first_invisible_character(pattern) {
            self.report_with_data(
                "no-invisible-character",
                "unexpected",
                DiagnosticData {
                    char_text: Some(mention_char(ch)),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(escape) = first_uppercase_hex_escape(pattern) {
            self.report_with_data(
                "letter-case",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(escape)),
                    replacement: Some(CompactString::from(escape.to_ascii_lowercase().as_str())),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some((escape, replacement)) = first_hex_x_escape(pattern) {
            self.report_with_data(
                "hexadecimal-escape",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(escape)),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some((escape, replacement)) = first_fixed_unicode_escape(pattern) {
            self.report_with_data(
                "unicode-escape",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(escape)),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if analysis.has_empty_lookaround {
            self.report("no-empty-lookarounds-assertion", "unexpected", span);
        }
        if let Some(ch) = analysis.first_useless_single_literal_class {
            let mut original = CompactString::new("[");
            original.push(ch);
            original.push(']');
            let mut replacement = CompactString::new("");
            replacement.push(ch);
            self.report_with_data(
                "no-useless-character-class",
                "unexpected",
                DiagnosticData {
                    expr: Some(original),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if pattern_has_empty_string_literal(pattern) {
            self.report("no-empty-string-literal", "unexpected", span);
        }
        if let Some(ch) = analysis.first_useless_range {
            let mut text = CompactString::new("");
            text.push(ch);
            text.push('-');
            text.push(ch);
            let mut replacement = CompactString::new("");
            replacement.push(ch);
            self.report_with_data(
                "no-useless-range",
                "unexpected",
                DiagnosticData {
                    expr: Some(text),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if analysis.has_optional_assertion {
            self.report("no-optional-assertion", "unexpected", span);
        }
        if analysis.has_confusing_quantifier {
            self.report("confusing-quantifier", "unexpected", span);
        }
        if let Some((start, end)) = analysis.first_obscure_range {
            let mut text = CompactString::new("");
            text.push(start);
            text.push('-');
            text.push(end);
            self.report_with_data(
                "no-obscure-range",
                "unexpected",
                DiagnosticData {
                    expr: Some(text),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(ch) = analysis.first_dupe_class_literal {
            let mut text = CompactString::new("");
            text.push(ch);
            self.report_with_data(
                "no-dupe-characters-character-class",
                "unexpected",
                DiagnosticData {
                    expr: Some(text),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some((start, end)) = analysis.first_collapsible_run {
            let mut original = CompactString::new("");
            // Push the inclusive run as the original text for the diagnostic.
            let mut cursor = start as u8;
            while cursor <= end as u8 {
                original.push(cursor as char);
                cursor += 1;
            }
            let mut replacement = CompactString::new("");
            replacement.push(start);
            replacement.push('-');
            replacement.push(end);
            self.report_with_data(
                "prefer-range",
                "unexpected",
                DiagnosticData {
                    expr: Some(original),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some((escape, replacement)) = first_surrogate_pair_escape(pattern) {
            self.report_with_data(
                "prefer-unicode-codepoint-escapes",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(escape)),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
    }
}
