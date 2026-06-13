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
    duplicate_flag, find_class_end, first_control_character, first_fixed_unicode_escape,
    first_hex_x_escape, first_invisible_character, first_literal_control_character,
    first_non_standard_flag, first_numbered_backreference_with_named_group, first_octal_escape,
    first_surrogate_pair_escape, first_uppercase_hex_escape, first_useless_escape,
    first_useless_one_quantifier, group_prefix, mention_char, pattern_ends_with_lazy_quantifier,
    pattern_has_empty_string_literal, skip_escape, sorted_flags, string_literal_value_with_span,
};
use crate::pattern::PatternAnalysis;
use crate::scanner::Scanner;
use crate::types::DiagnosticData;

/// Returns `true` when `replacement` contains a `$` that is followed by a
/// character outside the valid replacement-reference set: digit, `&`, `'`,
/// `` ` ``, `<` (named-group reference), `$` (escaped dollar). A trailing `$`
/// at the very end of the string also counts. Used by
/// `prefer-escape-replacement-dollar-char`.
fn replacement_has_lone_dollar(replacement: &str) -> bool {
    let bytes = replacement.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let Some(&next) = bytes.get(index + 1) else {
            return true;
        };
        if next == b'$' {
            index += 2;
            continue;
        }
        if next.is_ascii_digit() || matches!(next, b'&' | b'\'' | b'`' | b'<') {
            index += 2;
            continue;
        }
        return true;
    }
    false
}

/// Count the number of capturing groups in a regex pattern string.
/// Skips character classes `[...]`, escape sequences `\X`, and non-capturing
/// prefixes `(?:`, `(?=`, `(?!`, `(?<=`, `(?<!`.
fn count_capture_groups(pattern: &str) -> u32 {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    let mut count = 0u32;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = skip_escape(bytes, index);
            }
            b'[' => {
                if let Some(close) = find_class_end(bytes, index) {
                    index = close + 1;
                } else {
                    index += 1;
                }
            }
            b'(' => {
                let prefix = group_prefix(bytes, index);
                if prefix.capturing {
                    count += 1;
                }
                index = prefix.next;
            }
            _ => {
                index += 1;
            }
        }
    }
    count
}

/// Returns `true` when the replacement string contains a `$0N` (N = 1-9)
/// backreference that refers to a capture group that does not exist in the
/// pattern. Only the leading-zero form is checked here; `$N` (N = 1-9)
/// references without a leading zero are not flagged by this function so as
/// to stay conservative on receivers/patterns we cannot fully type-resolve.
///
/// JS replacement-string semantics for the `$0N` form:
/// - `$0N` (N 1-9) → same as `$N`: refers to capture group N.
///   Flag when group N does not exist in the pattern.
/// - `$0` (bare, not followed by a digit 1-9) → always a literal `$0`.
///   Never flag.
/// - `$$` → escaped dollar, skip.
fn replacement_has_useless_dollar_zero_ref(replacement: &str, capture_count: u32) -> bool {
    let bytes = replacement.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let Some(&next) = bytes.get(index + 1) else {
            index += 1;
            continue;
        };
        if next == b'$' {
            // `$$` — escaped dollar, skip both bytes
            index += 2;
            continue;
        }
        if next == b'0' {
            // Check for `$0N` where N is 1-9
            if let Some(&after) = bytes.get(index + 2)
                && matches!(after, b'1'..=b'9')
            {
                let n = (after - b'0') as u32;
                if n > capture_count {
                    return true;
                }
                index += 3;
                continue;
            }
            // Bare `$0` (not followed by 1-9) or `$00...` — always literal, never flag.
            index += 1;
            continue;
        }
        index += 1;
    }
    false
}

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
        self.check_no_useless_dollar_replacements(call);
        self.check_prefer_escape_replacement_dollar_char(call);
    }

    /// `prefer-escape-replacement-dollar-char`: in JS replacement strings a
    /// literal `$` should be written as `$$` to avoid being mistaken for a
    /// pattern reference. The reverse — `$` followed by an unrecognised char —
    /// is already a literal at runtime but is almost always a copy-paste bug.
    /// Flag any `$` in a literal replacement string that is followed by a char
    /// outside the valid reference set (digit, `&`, `'`, `` ` ``, `<`, `$`).
    fn check_prefer_escape_replacement_dollar_char(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "replace" && method != "replaceAll" {
            return;
        }
        // Only fire when the receiver is a string literal.
        if !matches!(
            member.object.get_inner_expression(),
            Expression::StringLiteral(_)
        ) {
            return;
        }
        // Only fire when the first argument is a regex literal.
        let Some(arg0) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        if !matches!(arg0.get_inner_expression(), Expression::RegExpLiteral(_)) {
            return;
        }
        let Some(arg1) = call.arguments.get(1).and_then(Argument::as_expression) else {
            return;
        };
        let Expression::StringLiteral(replacement) = arg1.get_inner_expression() else {
            return;
        };
        if !replacement_has_lone_dollar(replacement.value.as_str()) {
            return;
        }
        self.report(
            "prefer-escape-replacement-dollar-char",
            "unexpected",
            call.span,
        );
    }

    /// `no-useless-dollar-replacements`: in JavaScript replacement strings the
    /// `$0N` (N = 1-9) form is a backreference to capture group N. Flag when
    /// the referenced group does not exist in the pattern. Bare `$0` (not
    /// followed by a non-zero digit) is always a literal and is never flagged.
    ///
    /// Only the leading-zero form is checked here. Plain `$N` (N = 1-9)
    /// references require receiver-type tracking to avoid false positives on
    /// unknown variables (e.g. `str.replace(/./, '$1')`) and are deferred.
    /// Only literal `RegExp` first arguments are handled; constructor calls
    /// and variable-held patterns are deferred.
    fn check_no_useless_dollar_replacements(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "replace" && method != "replaceAll" {
            return;
        }
        let Some(arg1) = call.arguments.get(1).and_then(Argument::as_expression) else {
            return;
        };
        let Expression::StringLiteral(replacement) = arg1.get_inner_expression() else {
            return;
        };
        // First argument must be a literal RegExp so we know the group count.
        let Some(arg0) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Expression::RegExpLiteral(regex) = arg0.get_inner_expression() else {
            return;
        };
        let pattern = regex.regex.pattern.text.as_str();
        let capture_count = count_capture_groups(pattern);
        if !replacement_has_useless_dollar_zero_ref(replacement.value.as_str(), capture_count) {
            return;
        }
        self.report("no-useless-dollar-replacements", "unexpected", call.span);
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
        self.check_pattern_rules(pattern, flags, span, is_constructor);
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

    fn check_pattern_rules(
        &mut self,
        pattern: &str,
        flags: &str,
        span: Span,
        is_constructor: bool,
    ) {
        let mut analysis = PatternAnalysis::new();
        analysis.scan(pattern);

        // `no-useless-flag` (narrow form): the `s` flag only affects the
        // matching of `.`; the `m` flag only affects `^` and `$`. If neither
        // syntax appears in the pattern, the flag is provably inert. Other
        // useless-flag shapes (e.g. `i` on a pattern with no letters) are
        // intentionally deferred — they need a richer case-class analysis.
        if flags.contains('s') && !analysis.has_unescaped_dot {
            let mut text = CompactString::new("");
            text.push('s');
            self.report_with_data(
                "no-useless-flag",
                "unexpected",
                DiagnosticData {
                    flag: Some(text),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if flags.contains('m') && !analysis.has_unescaped_anchor {
            let mut text = CompactString::new("");
            text.push('m');
            self.report_with_data(
                "no-useless-flag",
                "unexpected",
                DiagnosticData {
                    flag: Some(text),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if analysis.has_case_pair_class && !flags.contains('i') {
            self.report("use-ignore-case", "unexpected", span);
        }
        if analysis.has_useless_non_capturing_group {
            self.report("no-useless-non-capturing-group", "unexpected", span);
        }
        if analysis.has_preferable_quantifier_group {
            self.report("prefer-quantifier", "unexpected", span);
        }
        if let Some(ch) = analysis.first_useless_string_literal {
            let mut original = CompactString::new("\\q{");
            original.push(ch);
            original.push('}');
            let mut replacement = CompactString::new("");
            replacement.push(ch);
            self.report_with_data(
                "grapheme-string-literal",
                "unexpected",
                DiagnosticData {
                    expr: Some(original.clone()),
                    replacement: Some(replacement.clone()),
                    ..DiagnosticData::default()
                },
                span,
            );
            // `no-useless-string-literal` covers the same single-character
            // `\q{X}` case from a slightly different angle (the string-literal
            // wrapper is dropped because the bare char already works in
            // v-mode classes); fire both rules so users can enable them
            // independently.
            self.report_with_data(
                "no-useless-string-literal",
                "unexpected",
                DiagnosticData {
                    expr: Some(original),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if analysis.has_unsorted_class_elements {
            self.report("sort-character-class-elements", "unexpected", span);
        }
        if analysis.has_trivially_nested_assertion {
            self.report("no-trivially-nested-assertion", "unexpected", span);
        }
        if analysis.has_extra_lookaround_assertion {
            self.report("no-extra-lookaround-assertions", "unexpected", span);
        }
        if analysis.has_trivially_nested_quantifier {
            self.report("no-trivially-nested-quantifier", "unexpected", span);
        }
        if analysis.has_preferable_character_class {
            self.report("prefer-character-class", "unexpected", span);
        }

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
            // When the pattern comes from a RegExp constructor argument, the six
            // characters with well-known named JS string escapes (\0 \t \n \v \f \r)
            // are delivered as literal bytes by the JS escape (e.g. '\n' → U+000A).
            // Upstream marks these valid (new RegExp('\n') is accepted), so suppress
            // here. For regex literals ALL control characters must be flagged.
            // Known gap: hex-escaped constructor args like new RegExp('\x0a') still
            // reach us as the literal char and would be suppressed — acceptable.
            let named_escape = matches!(ch, '\0' | '\t'..='\r');
            if !(is_constructor && named_escape) {
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
        }
        if let Some(ch) = first_literal_control_character(pattern) {
            // When the pattern comes from a RegExp constructor argument (not a
            // regex literal), the six characters that have well-known named
            // regex escapes (\0 \t \n \v \f \r) are written as JS string
            // escape sequences by the author (e.g. '\t') and are already
            // conceptually "named". Flagging them here would produce false
            // positives — upstream marks `new RegExp('\t')` as valid.
            // For regex literals, ALL literal control characters must be
            // escaped, because the author could have written \t instead of a
            // raw tab inside /.../.
            // The named-escape set is \0 (U+0000) and \t \n \v \f \r
            // (U+0009..=U+000D, contiguous).
            let named_escape = matches!(ch, '\0' | '\t'..='\r');
            if !(is_constructor && named_escape) {
                self.report_with_data(
                    "control-character-escape",
                    "unexpected",
                    DiagnosticData {
                        char_text: Some(mention_char(ch)),
                        ..DiagnosticData::default()
                    },
                    span,
                );
            }
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
        if pattern_ends_with_lazy_quantifier(pattern) {
            self.report("no-lazy-ends", "unexpected", span);
        }
        if let Some(text) = first_useless_one_quantifier(pattern) {
            self.report_with_data(
                "no-useless-quantifier",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(text)),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(text) = first_numbered_backreference_with_named_group(pattern) {
            self.report_with_data(
                "prefer-named-backreference",
                "unexpected",
                DiagnosticData {
                    expr: Some(CompactString::from(text)),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        if let Some(byte) = first_useless_escape(pattern) {
            let mut text = CompactString::new("\\");
            text.push(byte as char);
            let mut replacement = CompactString::new("");
            replacement.push(byte as char);
            self.report_with_data(
                "no-useless-escape",
                "unexpected",
                DiagnosticData {
                    expr: Some(text),
                    replacement: Some(replacement),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
        // Surrogate-pair escapes are only meaningful with the `u`/`v` flag;
        // without it `\uHHHH\uHHHH` is two independent code units, so upstream
        // leaves them alone.
        if (flags.contains('u') || flags.contains('v'))
            && let Some((escape, replacement)) = first_surrogate_pair_escape(pattern)
        {
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
