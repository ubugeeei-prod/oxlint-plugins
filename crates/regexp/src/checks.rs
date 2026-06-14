//! Regexp-specific diagnostic checks: literals, constructors, flags, and
//! pattern-level rules. These methods are reached through the AST traversal in
//! `traversal.rs` and rely on helpers in `helpers.rs` / `pattern.rs`.

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, CallExpression, Expression, NewExpression, RegExpLiteral, StaticMemberExpression,
};
use oxc_regular_expression::{ConstructorParser, Options as RegExpOptions};
use oxc_span::Span;
use oxlint_plugins_carton::CompactString;

use crate::helpers::{
    GroupReplacementRef, duplicate_flag, find_class_end, first_control_character,
    first_fixed_unicode_escape, first_invisible_character, first_literal_control_character,
    first_non_standard_flag, first_numbered_backreference_with_named_group, first_octal_escape,
    first_strict_violation, first_surrogate_pair_escape, first_unicode_escape_as_hex,
    first_uppercase_hex_escape, first_useless_escape, first_useless_one_quantifier, group_prefix,
    has_assertion_contradiction, has_mergeable_quantifier_concatenation,
    has_preferable_set_operation, has_simplifiable_set_operation, has_standalone_backslash,
    has_unnecessary_general_category_key, has_useless_set_operand, has_useless_word_boundary,
    mention_char, pattern_ends_with_lazy_quantifier,
    pattern_has_capturing_group_and_no_backreference, pattern_has_empty_string_literal,
    pattern_is_safe_to_add_i_flag, prefer_lookaround_groups, skip_escape, sorted_flags,
    string_literal_value_with_span,
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

/// Builds the literal replacement token for a group reference: `$N` for a
/// numbered group or `$<name>` for a named one.
fn group_ref_token(reference: &GroupReplacementRef) -> CompactString {
    match reference {
        GroupReplacementRef::Numbered(n) => {
            let mut token = CompactString::new("$");
            // Single-digit groups only ever reach here (`prefer_lookaround_groups`
            // yields groups 1 and 2), but format defensively.
            let mut buf = [0u8; 10];
            let mut cursor = buf.len();
            let mut value = *n;
            if value == 0 {
                cursor -= 1;
                buf[cursor] = b'0';
            } else {
                while value > 0 {
                    cursor -= 1;
                    buf[cursor] = b'0' + (value % 10) as u8;
                    value /= 10;
                }
            }
            if let Ok(text) = std::str::from_utf8(&buf[cursor..]) {
                token.push_str(text);
            }
            token
        }
        GroupReplacementRef::Named(name) => {
            let mut token = CompactString::new("$<");
            token.push_str(name.as_str());
            token.push('>');
            token
        }
    }
}

/// Counts non-overlapping occurrences of `needle` in `haystack`. `needle` is
/// always non-empty here.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut rest = haystack;
    while let Some(pos) = rest.find(needle) {
        count += 1;
        rest = &rest[pos + needle.len()..];
    }
    count
}

/// Returns `true` when `replacement` begins with the group-1 reference token,
/// ends with the group-2 reference token, references group 1 exactly once and
/// group 2 exactly once, and the two tokens do not overlap. This is the
/// `prefer-lookaround` both-ends signature: each captured group is re-emitted
/// at the same extreme position it occupied in the pattern, so the capture is
/// pure assertion.
fn replacement_uses_refs_at_extremes(
    replacement: &str,
    ref1: &GroupReplacementRef,
    ref2: &GroupReplacementRef,
) -> bool {
    let token1 = group_ref_token(ref1);
    let token2 = group_ref_token(ref2);
    // Each reference must appear exactly once.
    if count_occurrences(replacement, token1.as_str()) != 1
        || count_occurrences(replacement, token2.as_str()) != 1
    {
        return false;
    }
    if !replacement.starts_with(token1.as_str()) || !replacement.ends_with(token2.as_str()) {
        return false;
    }
    // The two tokens must not overlap (e.g. a short replacement where start and
    // end coincide). Require the start token to end at or before the start of
    // the end token.
    let end_token_start = replacement.len().saturating_sub(token2.len());
    token1.len() <= end_token_start
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

/// Builds a bitmask of capture-group indices (1-based, bits 1-9) that are
/// named groups in `pattern`.  The bitmask is zero when there are no named
/// groups.  Uses the same scanning approach as
/// `first_numbered_backreference_with_named_group` in `helpers.rs`.
fn named_group_mask(pattern: &str) -> u32 {
    if !pattern.contains("(?<") {
        return 0;
    }
    let bytes = pattern.as_bytes();
    let mut mask: u32 = 0;
    let mut group_counter: u32 = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'[' {
            if let Some(close) = find_class_end(bytes, index) {
                index = close + 1;
            } else {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index).max(index + 1);
            continue;
        }
        if bytes[index] == b'(' {
            let gp = group_prefix(bytes, index);
            if gp.capturing {
                group_counter += 1;
                if gp.named && group_counter <= 9 {
                    mask |= 1 << group_counter;
                }
            }
            index = gp.next;
            continue;
        }
        index += 1;
    }
    mask
}

/// Returns `true` when `replacement` contains a `$N` backreference (N = 1-9)
/// where capture group N is a *named* group according to `named_mask`.
/// `$$` (escaped dollar) is intentionally skipped.  Groups beyond 9 and the
/// `$0` form are out of scope for this rule.
fn replacement_has_named_group_numeric_ref(replacement: &str, named_mask: u32) -> bool {
    if named_mask == 0 {
        return false;
    }
    let bytes = replacement.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'$' {
            let next = bytes[index + 1];
            if next == b'$' {
                index += 2;
                continue;
            }
            if matches!(next, b'1'..=b'9') {
                let group_num = (next - b'0') as u32;
                if named_mask & (1 << group_num) != 0 {
                    return true;
                }
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
        if self.is_global_regexp_callee(&call.callee) {
            self.check_regexp_constructor(call.span, &call.arguments);
        }
        self.check_prefer_regexp_exec(call);
        self.check_prefer_regexp_test(call);
        self.check_no_missing_g_flag(call);
        self.check_prefer_named_replacement(call);
        self.check_no_useless_dollar_replacements(call);
        self.check_prefer_escape_replacement_dollar_char(call);
        self.check_no_unused_capturing_group(call);
        self.check_prefer_lookaround(call);
    }

    /// `prefer-lookaround` (narrow form, both-ends case): a `.replace` /
    /// `.replaceAll` whose regex is `(B1)MID(B2)` and whose replacement just
    /// re-emits `$1` at the very start and `$2` at the very end can use
    /// lookaround assertions instead: `(?<=B1)MID(?=B2)`.
    ///
    /// This handles only the provably-safe structural shape recognised by
    /// [`prefer_lookaround_groups`] (plain fixed-length group bodies, non-empty
    /// plain middle, differing boundary bytes, exactly two groups, no `g`
    /// flag). The replacement must:
    /// * be a string literal,
    /// * begin with the reference to group 1 (`$1` or `$<name1>`),
    /// * end with the reference to group 2 (`$2` or `$<name2>`),
    /// * reference group 1 exactly once and group 2 exactly once.
    ///
    /// Any deviation (other `$N`, reuse, extra groups, `g` flag, dynamic
    /// replacement) is left alone to stay valid-sound against the upstream
    /// suite, whose valid cases hinge on overlapping-match safety that this
    /// narrow shape sidesteps.
    fn check_prefer_lookaround(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "replace" && method != "replaceAll" {
            return;
        }
        let Some(arg0) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Expression::RegExpLiteral(literal) = arg0.get_inner_expression() else {
            return;
        };
        let flags = literal
            .raw
            .as_ref()
            .and_then(|raw| raw.as_str().rsplit_once('/').map(|(_, flags)| flags))
            .unwrap_or("");
        // The `g` flag enables overlapping global matches whose safety the
        // narrow structural shape cannot guarantee; skip it.
        if flags.contains('g') {
            return;
        }
        let Some(arg1) = call.arguments.get(1).and_then(Argument::as_expression) else {
            return;
        };
        let Expression::StringLiteral(replacement) = arg1.get_inner_expression() else {
            return;
        };
        let pattern = literal.regex.pattern.text.as_str();
        let Some((ref1, ref2)) = prefer_lookaround_groups(pattern) else {
            return;
        };
        if replacement_uses_refs_at_extremes(replacement.value.as_str(), &ref1, &ref2) {
            self.report("prefer-lookaround", "preferLookarounds", call.span);
        }
    }

    /// `prefer-result-array-groups` (narrow form): when a match-array result
    /// of a regex that has *named* capture groups is indexed numerically
    /// (`arr[N]`) and index `N` maps to a named group, the named-group access
    /// (`arr.groups.<name>`) is clearer.
    ///
    /// Sound, single-hop subset (no full type tracker):
    /// * Inline: `<regexLiteral>.exec(x)[N]` / `...?.[N]`.
    /// * One variable hop: `arr[N]` where `arr` is declared `const/let/var arr =
    ///   <src>` and `<src>` is `<regexLiteral>.exec(x)` or
    ///   `<knownString>.match(<regexLiteral>)`.
    ///
    /// Soundness guards:
    /// * `N` must be a non-negative integer literal `>= 1` (index 0 is the whole
    ///   match, never a group).
    /// * Group `N` of the literal must be a *named* group (bit `N` set in the
    ///   named-group mask). Unnamed groups have no named alternative.
    /// * For `.match`, the regex must NOT carry the `g` flag — a global
    ///   `.match` returns a flat string array with no group indexing.
    ///
    /// Anything that is not one of these exact shapes (unknown receiver chains,
    /// multi-hop aliases, `.groups` access, non-numeric index) is left alone.
    pub(crate) fn check_prefer_result_array_groups(
        &mut self,
        member: &'a oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        // The index must be a numeric literal `N >= 1`.
        let Expression::NumericLiteral(index) = member.expression.get_inner_expression() else {
            return;
        };
        let value = index.value;
        if value < 1.0 || value.fract() != 0.0 || value > 31.0 {
            return;
        }
        let n = value as u32;

        let Some(mask) = self.array_source_named_mask(&member.object) else {
            return;
        };
        if mask & (1 << n) != 0 {
            self.report("prefer-result-array-groups", "unexpected", member.span);
        }
    }

    /// Returns the named-group bitmask (bits 1..=9, see [`named_group_mask`]) of
    /// the regex literal backing `expr` when `expr` is a recognised match-array
    /// source, otherwise `None`.
    ///
    /// Recognised sources (one hop max):
    /// * `<regexLiteral>.exec(...)` — flags irrelevant to result shape.
    /// * `<knownString>.match(<regexLiteral>)` — only when the literal has no
    ///   `g` flag.
    /// * An identifier resolving to a `const/let/var` initialised with one of
    ///   the above.
    fn array_source_named_mask(&self, expr: &Expression<'a>) -> Option<u32> {
        match expr.get_inner_expression() {
            Expression::CallExpression(call) => self.call_result_named_mask(call),
            Expression::Identifier(ident) => {
                let reference_id = ident.reference_id.get()?;
                let symbol_id = self.scoping.get_reference(reference_id).symbol_id()?;
                let decl_node_id = self.scoping.symbol_declaration(symbol_id);
                let AstKind::VariableDeclarator(declarator) =
                    self.nodes.get_node(decl_node_id).kind()
                else {
                    return None;
                };
                let init = declarator.init.as_ref()?;
                match init.get_inner_expression() {
                    Expression::CallExpression(call) => self.call_result_named_mask(call),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Returns the named-group bitmask of the regex literal driving a
    /// `.exec()` / `.match()` call, when the call matches the narrow shapes
    /// (see [`Self::check_prefer_result_array_groups`]). `None` otherwise.
    fn call_result_named_mask(&self, call: &'a CallExpression<'a>) -> Option<u32> {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return None;
        };
        let method = member.property.name.as_str();
        if method == "exec" {
            // `<regexLiteral>.exec(x)` — the receiver is the regex literal.
            let Expression::RegExpLiteral(literal) = member.object.get_inner_expression() else {
                return None;
            };
            let mask = named_group_mask(literal.regex.pattern.text.as_str());
            (mask != 0).then_some(mask)
        } else if method == "match" {
            // `<knownString>.match(<regexLiteral>)`.
            if !self.receiver_is_known_string(&member.object) {
                return None;
            }
            let arg0 = call.arguments.first().and_then(Argument::as_expression)?;
            let Expression::RegExpLiteral(literal) = arg0.get_inner_expression() else {
                return None;
            };
            let flags = literal
                .raw
                .as_ref()
                .and_then(|raw| raw.as_str().rsplit_once('/').map(|(_, flags)| flags))
                .unwrap_or("");
            if flags.contains('g') {
                return None;
            }
            let mask = named_group_mask(literal.regex.pattern.text.as_str());
            (mask != 0).then_some(mask)
        } else {
            None
        }
    }

    /// `no-unused-capturing-group` (narrow form): a capturing group whose
    /// capture is provably never read. The only fully-sound, literal-only case
    /// handled here is a direct `RegExp#test` call on a regex literal:
    /// `/(...)/.test(x)`. `RegExp.prototype.test` returns only a boolean, so
    /// its capturing groups can never be observed.
    ///
    /// Soundness constraints:
    /// * The receiver must be a *regex literal* (not a variable that might be
    ///   reused elsewhere, e.g. in a `.replace` that reads `$1`).
    /// * The method must be exactly `test`.
    /// * The pattern must contain at least one capturing group and no in-pattern
    ///   backreference (`\1`/`\k<name>`) — a backref counts as a use.
    ///
    /// Broader cases (`.search` argument, `.match` whose array is unindexed,
    /// `.replace` with no `$N`, variable-held literals) require data-flow
    /// analysis and are deferred to stay valid-sound against the upstream suite.
    fn check_no_unused_capturing_group(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        if member.property.name != "test" {
            return;
        }
        let Expression::RegExpLiteral(literal) = member.object.get_inner_expression() else {
            return;
        };
        let pattern = literal.regex.pattern.text.as_str();
        if pattern_has_capturing_group_and_no_backreference(pattern) {
            self.report(
                "no-unused-capturing-group",
                "unusedCapturingGroup",
                literal.span,
            );
        }
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

    /// `prefer-named-replacement`: when `<string>.replace(<regexp with named
    /// captures>, "...$N...")` uses a numbered backreference `$N` where group N
    /// is itself a named capture, the named form `$<name>` is clearer.
    ///
    /// Two conditions must both hold before we report:
    /// 1. The receiver is a statically-known string (string literal, no-expr
    ///    template, or a `const`/`let`/`var` initialised with one). An unknown
    ///    receiver (free variable, call result, …) might not be a `String` at
    ///    all and is left unreported.
    /// 2. The `$N` in the replacement refers to a group index that is itself a
    ///    *named* group in the regex literal. A `$N` that points at an unnamed
    ///    group has no named alternative and must not be flagged.
    ///
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
        // Condition 1: receiver must be a statically-known string.
        if !self.receiver_is_known_string(&member.object) {
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
        let pattern = literal.regex.pattern.text.as_str();
        // Condition 2: build named-group mask; bail early when there are none.
        let mask = named_group_mask(pattern);
        if mask == 0 {
            return;
        }
        let Some(arg1) = call.arguments.get(1).and_then(Argument::as_expression) else {
            return;
        };
        let Expression::StringLiteral(replacement) = arg1.get_inner_expression() else {
            return;
        };
        if !replacement_has_named_group_numeric_ref(replacement.value.as_str(), mask) {
            return;
        }
        self.report("prefer-named-replacement", "unexpected", call.span);
    }

    /// `no-missing-g-flag`: `<expr>.matchAll(<regexp without 'g'>)` and
    /// `<expr>.replaceAll(<regexp without 'g'>, ...)` throw at runtime (the
    /// engine requires the global flag). Flag the literal-regexp case
    /// statically; constructor calls are deferred for the same type-info
    /// reason as `prefer-regexp-exec`.
    ///
    /// Only reports when the receiver is statically known to be a string
    /// (string literal, no-expression template literal, or a variable
    /// initialised with one of those). An unknown/free receiver (e.g.
    /// `unknown.replaceAll(/foo/, 'bar')`) is left unreported because it may
    /// not be a `String` and therefore may not enforce the `g` flag at all.
    fn check_no_missing_g_flag(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "matchAll" && method != "replaceAll" {
            return;
        }
        // Only report when the receiver is a known string value.  An unknown
        // receiver (free variable, call result, etc.) might not be a String
        // at all, so we must not enforce the `g` flag.
        if !self.receiver_is_known_string(&member.object) {
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

    /// `prefer-regexp-test` (narrow form): flag `.exec()` / `.match()` calls
    /// whose result is only consumed as a boolean, recommending `RegExp#test`
    /// instead.
    ///
    /// Narrow constraints (no type-tracker available):
    ///
    /// * For `.exec()`: the receiver must NOT be a statically-known string
    ///   value.  A string receiver means it is `String.prototype.exec` (which
    ///   does not exist as a regexp method), so flagging it would be a false
    ///   positive.  The check is purely structural: if `receiver_is_known_string`
    ///   returns `true` we skip.
    ///
    /// * For `.match()`: the receiver MUST be a statically-known string value
    ///   (to confirm this is `String.prototype.match`, not `RegExp[@@match]`),
    ///   and the single argument must be a RegExp literal that does NOT have the
    ///   `g` flag (a `g`-flagged `.match()` returns all matches as an array and
    ///   is not equivalent to `.test()`).
    ///
    /// Both variants are only reported when `self.in_boolean_ctx` is `true` —
    /// i.e. the call's result is provably consumed as a boolean (the test of an
    /// `if`/`while`/ternary, the operand of `!`, the argument to `Boolean()`,
    /// one side of `&&`/`||`, or a `=== null` / `!== null` comparison).
    fn check_prefer_regexp_test(&mut self, call: &'a CallExpression<'a>) {
        if !self.in_boolean_ctx {
            return;
        }
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();

        if method == "exec" {
            if call.arguments.len() != 1 {
                return;
            }
            // Skip when the receiver is a known string — that would be
            // `String.prototype.exec` (no-op / wrong receiver), not
            // `RegExp.prototype.exec`.
            if self.receiver_is_known_string(&member.object) {
                return;
            }
            self.report("prefer-regexp-test", "disallow", call.span);
        } else if method == "match" {
            if call.arguments.len() != 1 {
                return;
            }
            // The receiver must be a known string so we know this is
            // `String.prototype.match` (not `RegExp[@@match]`).
            if !self.receiver_is_known_string(&member.object) {
                return;
            }
            // The argument must be a RegExp literal without the `g` flag.
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
            self.report("prefer-regexp-test", "disallow", call.span);
        }
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
        if self.is_global_regexp_callee(&new_expression.callee) {
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
        // Determine whether this regex literal is used as a complete (whole)
        // pattern — i.e. actually matched against a string.  The pre-pass in
        // `usage.rs` collected the span-starts of all such literals.
        let used_as_whole = self.whole_pattern_regex_spans.contains(&literal.span.start);
        self.check_regexp(
            pattern,
            flags,
            literal.span,
            false,
            None,
            None,
            false,
            used_as_whole,
        );
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
        // `flags_arg_expr` is the second argument (if any) as an expression.
        let flags_arg_expr = arguments.get(1).and_then(Argument::as_expression);
        // `flags` is `Some(...)` only when the flags argument is a string literal.
        let flags = flags_arg_expr.and_then(string_literal_value_with_span);
        // `flags_is_non_literal` is true when a flags argument is present but is
        // not a string literal (e.g. an identifier, binary expression, or member
        // expression). In that case we cannot statically know the flag set, so
        // `require-unicode-regexp` must not fire.
        let flags_is_non_literal = flags_arg_expr.is_some() && flags.is_none();
        let flags_value = flags.map_or("", |(value, _)| value);
        self.check_regexp(
            pattern,
            flags_value,
            span,
            true,
            Some(pattern_span),
            flags.map(|(_, span)| span),
            flags_is_non_literal,
            // RegExp constructor calls produce patterns that are typically used
            // as building blocks (partial), so `no-lazy-ends` is not fired.
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn check_regexp(
        &mut self,
        pattern: &str,
        flags: &str,
        span: Span,
        is_constructor: bool,
        pattern_span: Option<Span>,
        flags_span: Option<Span>,
        // `true` when this is a constructor call and the flags argument is a
        // non-literal expression (identifier, binary, member, etc.). In that
        // case we cannot statically determine the flags, so the
        // `require-unicode-regexp` and `require-unicode-sets-regexp` checks
        // are skipped to avoid false positives.
        flags_is_non_literal: bool,
        // `true` when the regex is provably used as a complete (whole) pattern —
        // directly called via `.test()` / `.exec()` / etc., or via a variable
        // that is used that way and is not exported.  Controls `no-lazy-ends`.
        used_as_whole: bool,
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

        self.check_flag_style(flags, span, flags_is_non_literal);
        self.check_pattern_rules(pattern, flags, span, is_constructor, used_as_whole);
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

    fn check_flag_style(&mut self, flags: &str, span: Span, flags_is_non_literal: bool) {
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
        // When the flags argument is a non-literal expression (identifier,
        // binary, member, etc.) we cannot statically know the flag set, so
        // skip the unicode-presence checks to avoid false positives.
        if !flags_is_non_literal {
            if !flags.contains('u') && !flags.contains('v') {
                self.report("require-unicode-regexp", "require", span);
            }
            if !flags.contains('v') {
                self.report("require-unicode-sets-regexp", "require", span);
            }
        }
    }

    fn check_pattern_rules(
        &mut self,
        pattern: &str,
        flags: &str,
        span: Span,
        is_constructor: bool,
        used_as_whole: bool,
    ) {
        let mut analysis = PatternAnalysis::new();
        analysis.scan(pattern, flags.contains('v'));

        // `strict` (narrow form): only fire on non-`u`/non-`v` patterns.
        // The `u`/`v` flags turn on strict parsing automatically, so those
        // patterns are already validated by the parser.
        if !flags.contains('u')
            && !flags.contains('v')
            && let Some(message_id) = first_strict_violation(pattern)
        {
            self.report("strict", message_id, span);
        }

        // `no-useless-assertions` (narrow form): a `\b` / `\B` word-boundary
        // assertion sandwiched between two literal characters that share the
        // same word class. `\b` there can never match; `\B` there always
        // matches. Both are useless. Soundness is preserved by only inspecting
        // unambiguous literal neighbours (see `has_useless_word_boundary`).
        if has_useless_word_boundary(pattern) {
            self.report("no-useless-assertions", "unexpected", span);
        }

        // `optimal-quantifier-concatenation` (narrow form): two adjacent
        // quantified atoms on the same single element where at least one
        // quantifier is greedily unbounded can always be merged into a single
        // quantifier (e.g. `aa*` → `a+`, `\w*\w` → `\w+`). See
        // `has_mergeable_quantifier_concatenation` for soundness boundaries.
        if has_mergeable_quantifier_concatenation(pattern, flags.contains('v')) {
            self.report("optimal-quantifier-concatenation", "unexpected", span);
        }

        // `no-contradiction-with-assertion` (narrow form): a `\b` boundary
        // directly followed by a min-zero quantifier on a same-word-class
        // literal can never be entered (`/a\ba*-/`). See
        // `has_assertion_contradiction` for soundness boundaries.
        if has_assertion_contradiction(pattern) {
            self.report("no-contradiction-with-assertion", "unexpected", span);
        }

        // `no-useless-set-operand` (narrow form): a v-mode set operation
        // `[A&&B]` / `[A--B]` whose two shorthand operands make one operand
        // redundant (disjoint or subset). Only meaningful in v-mode. See
        // `has_useless_set_operand` for soundness boundaries.
        if flags.contains('v') && has_useless_set_operand(pattern) {
            self.report("no-useless-set-operand", "unexpected", span);
        }

        // `prefer-set-operation` (narrow form): a v-mode char lookaround
        // adjacent to a char element can be rewritten as a `[Y&&X]`/`[Y--X]`
        // set operation. See `has_preferable_set_operation` for soundness.
        if flags.contains('v') && has_preferable_set_operation(pattern) {
            self.report("prefer-set-operation", "unexpected", span);
        }

        // `simplify-set-operations` (narrow form): a v-mode `&&` intersection
        // with a negated nested-class operand can be simplified (subtraction or
        // De Morgan). See `has_simplifiable_set_operation` for boundaries.
        if flags.contains('v') && has_simplifiable_set_operation(pattern) {
            self.report("simplify-set-operations", "unexpected", span);
        }

        // `unicode-property` (narrow form): under the upstream default config
        // (`generalCategory: "never"`) an explicit General_Category key in a
        // `\p{gc=...}` / `\p{General_Category=...}` property escape is redundant
        // and can be dropped. Only this clearly-redundant key form is flagged.
        if has_unnecessary_general_category_key(pattern) {
            self.report("unicode-property", "unnecessaryGc", span);
        }

        // `no-potentially-useless-backreference` (narrow form): only flag the
        // syntactically clear case where group N is directly followed by `?`
        // or `*` (so the group may not have matched at the point of the
        // backref). Alternative-branch cases require reachability analysis and
        // are deferred.
        if analysis.has_potentially_useless_backreference {
            self.report(
                "no-potentially-useless-backreference",
                "potentiallyUselessBackreference",
                span,
            );
        }

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
        if analysis.has_case_pair_class
            && !flags.contains('i')
            && pattern_is_safe_to_add_i_flag(pattern, flags)
        {
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
        if analysis.has_unsorted_alternatives {
            self.report("sort-alternatives", "unexpected", span);
        }
        if analysis.has_preferable_predefined_assertion {
            self.report("prefer-predefined-assertion", "unexpected", span);
        }
        if analysis.has_suboptimal_lookaround_quantifier {
            self.report("optimal-lookaround-quantifier", "unexpected", span);
        }
        if analysis.has_dupe_disjunctions {
            self.report("no-dupe-disjunctions", "unexpected", span);
        }
        if analysis.has_useless_backreference {
            self.report("no-useless-backreference", "unexpected", span);
        }
        if analysis.has_negation_shorthand {
            self.report("negation", "unexpected", span);
        }
        if analysis.has_useless_lazy {
            self.report("no-useless-lazy", "unexpected", span);
        }
        if analysis.has_misleading_unicode_character {
            self.report("no-misleading-unicode-character", "unexpected", span);
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
            // When the pattern comes from a RegExp constructor argument, the six
            // characters with well-known named JS string escapes (\0 \t \n \v \f \r)
            // are delivered as literal bytes by the JS escape (e.g. '\t' → U+0009).
            // Upstream marks these valid (new RegExp('\t') is accepted), so suppress
            // here. For regex literals ALL invisible characters must be flagged.
            // Known gap: hex-escaped constructor args like new RegExp('\x09') still
            // reach us as the literal char and would be suppressed — acceptable.
            let named_escape = matches!(ch, '\0' | '\t'..='\r');
            if !(is_constructor && named_escape) {
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
        if let Some((escape, replacement)) = first_unicode_escape_as_hex(pattern) {
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
        // `no-lazy-ends`: only fire when the regex is provably used as a
        // whole (complete) pattern, matching upstream `ignorePartial: true`
        // (the default).  A bare literal or an exported binding is "unknown /
        // partial" and should not be flagged.
        if used_as_whole && pattern_ends_with_lazy_quantifier(pattern) {
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
        // `no-standalone-backslash`: in non-`u`/non-`v` mode the engine
        // silently accepts `\c[non-letter]` as a literal backslash. This is
        // almost certainly unintentional — the author probably intended a
        // control-character escape `\cX` (which requires a letter after `\c`).
        // Narrow form: only flag when `u`/`v` are absent (with those flags the
        // pattern is a parse error that `no-invalid-regexp` already catches).
        if !flags.contains('u') && !flags.contains('v') && has_standalone_backslash(pattern) {
            self.report("no-standalone-backslash", "unexpected", span);
        }
    }
}
