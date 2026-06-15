//! Token-stream stylistic rules.
//!
//! These rules reason purely about the lexed [`Token`] stream (see
//! [`super::lexer`]) and the raw gaps between adjacent tokens. They need no AST,
//! which keeps them on the "single native scan" fast path: one tokenization per
//! source feeds every enabled token rule.
//!
//! Each rule mirrors the corresponding `@stylistic` rule's default behaviour and
//! its primary options. Whitespace-only gaps are recovered from the source text
//! between two tokens; a gap that contains a line break is generally treated as
//! "multiline" and left alone, matching `@stylistic`'s line-break exemptions.

use serde_json::Value;

use crate::LintDiagnostic;

use super::context::{
    Scan, has_newline, is_whitespace, option_keyword, option_object_bool, punct_is,
    report_missing_space, report_replace, report_unexpected_space,
};
use super::helpers::push_diagnostic;
use super::lexer::TokenKind;

// ---------------------------------------------------------------------------
// arrow-spacing
// ---------------------------------------------------------------------------

pub(crate) fn check_arrow_spacing(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "arrow-spacing";
    let before = option_object_bool(options, "before", true);
    let after = option_object_bool(options, "after", true);

    for index in 0..scan.tokens().len() {
        let token = &scan.tokens()[index];
        if !punct_is(token, scan.source(), "=>") {
            continue;
        }
        if let Some(prev) = scan
            .tokens()
            .get(index.wrapping_sub(1))
            .filter(|_| index > 0)
        {
            let gap = scan.gap(prev, token);
            if before && gap.is_empty() {
                report_missing_space(
                    diagnostics,
                    RULE,
                    "expectedBefore",
                    "Missing space before =>.",
                    token.start,
                );
            } else if !before && is_whitespace(gap) {
                report_unexpected_space(
                    diagnostics,
                    RULE,
                    "unexpectedBefore",
                    "Unexpected space before =>.",
                    prev.end,
                    token.start,
                );
            }
        }
        if let Some(next) = scan.tokens().get(index + 1) {
            let gap = scan.gap(token, next);
            if after && gap.is_empty() {
                report_missing_space(
                    diagnostics,
                    RULE,
                    "expectedAfter",
                    "Missing space after =>.",
                    token.end,
                );
            } else if !after && is_whitespace(gap) {
                report_unexpected_space(
                    diagnostics,
                    RULE,
                    "unexpectedAfter",
                    "Unexpected space after =>.",
                    token.end,
                    next.start,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// comma-spacing
// ---------------------------------------------------------------------------

pub(crate) fn check_comma_spacing(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "comma-spacing";
    let before = option_object_bool(options, "before", false);
    let after = option_object_bool(options, "after", true);

    for index in 0..scan.tokens().len() {
        let comma = &scan.tokens()[index];
        if !punct_is(comma, scan.source(), ",") {
            continue;
        }
        if index > 0 {
            if let Some(prev) = scan.tokens().get(index - 1) {
                let gap = scan.gap(prev, comma);
                if !has_newline(gap) {
                    if before && gap.is_empty() {
                        report_missing_space(
                            diagnostics,
                            RULE,
                            "missing",
                            "A space is required before ','.",
                            comma.start,
                        );
                    } else if !before && is_whitespace(gap) {
                        report_unexpected_space(
                            diagnostics,
                            RULE,
                            "unexpected",
                            "There should be no space before ','.",
                            prev.end,
                            comma.start,
                        );
                    }
                }
            }
        }
        if let Some(next) = scan.tokens().get(index + 1) {
            // Holes and closing delimiters never need a following space.
            if punct_is(next, scan.source(), ",")
                || punct_is(next, scan.source(), ")")
                || punct_is(next, scan.source(), "]")
                || punct_is(next, scan.source(), "}")
            {
                continue;
            }
            let gap = scan.gap(comma, next);
            if has_newline(gap) {
                continue;
            }
            if after && gap.is_empty() {
                report_missing_space(
                    diagnostics,
                    RULE,
                    "missing",
                    "A space is required after ','.",
                    comma.end,
                );
            } else if !after && is_whitespace(gap) {
                report_unexpected_space(
                    diagnostics,
                    RULE,
                    "unexpected",
                    "There should be no space after ','.",
                    comma.end,
                    next.start,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// semi-spacing
// ---------------------------------------------------------------------------

pub(crate) fn check_semi_spacing(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "semi-spacing";
    let before = option_object_bool(options, "before", false);
    let after = option_object_bool(options, "after", true);

    for index in 0..scan.tokens().len() {
        let semi = &scan.tokens()[index];
        if !punct_is(semi, scan.source(), ";") {
            continue;
        }
        if index > 0 {
            if let Some(prev) = scan.tokens().get(index - 1) {
                let gap = scan.gap(prev, semi);
                if !has_newline(gap) {
                    if before && gap.is_empty() {
                        report_missing_space(
                            diagnostics,
                            RULE,
                            "missing",
                            "A space is required before ';'.",
                            semi.start,
                        );
                    } else if !before && is_whitespace(gap) {
                        report_unexpected_space(
                            diagnostics,
                            RULE,
                            "unexpected",
                            "There should be no space before ';'.",
                            prev.end,
                            semi.start,
                        );
                    }
                }
            }
        }
        if let Some(next) = scan.tokens().get(index + 1) {
            // `for (;;)` and a `;` before `}` or `)` need no following space.
            if punct_is(next, scan.source(), ")")
                || punct_is(next, scan.source(), "}")
                || punct_is(next, scan.source(), ";")
            {
                continue;
            }
            let gap = scan.gap(semi, next);
            if has_newline(gap) {
                continue;
            }
            if after && gap.is_empty() {
                report_missing_space(
                    diagnostics,
                    RULE,
                    "missing",
                    "A space is required after ';'.",
                    semi.end,
                );
            } else if !after && is_whitespace(gap) {
                report_unexpected_space(
                    diagnostics,
                    RULE,
                    "unexpected",
                    "There should be no space after ';'.",
                    semi.end,
                    next.start,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// space-in-parens
// ---------------------------------------------------------------------------

pub(crate) fn check_space_in_parens(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "space-in-parens";
    let always = option_keyword(options, "never") == "always";

    for index in 0..scan.tokens().len() {
        let token = &scan.tokens()[index];
        if punct_is(token, scan.source(), "(") {
            let Some(next) = scan.tokens().get(index + 1) else {
                continue;
            };
            // Empty `()` is always exempt.
            if punct_is(next, scan.source(), ")") {
                continue;
            }
            let gap = scan.gap(token, next);
            if has_newline(gap) {
                continue;
            }
            if always && gap.is_empty() {
                report_missing_space(
                    diagnostics,
                    RULE,
                    "missingOpeningSpace",
                    "There must be a space after this paren.",
                    token.end,
                );
            } else if !always && is_whitespace(gap) {
                report_unexpected_space(
                    diagnostics,
                    RULE,
                    "rejectedOpeningSpace",
                    "There should be no space after this paren.",
                    token.end,
                    next.start,
                );
            }
        } else if punct_is(token, scan.source(), ")") && index > 0 {
            let Some(prev) = scan.tokens().get(index - 1) else {
                continue;
            };
            if punct_is(prev, scan.source(), "(") {
                continue;
            }
            let gap = scan.gap(prev, token);
            if has_newline(gap) {
                continue;
            }
            if always && gap.is_empty() {
                report_missing_space(
                    diagnostics,
                    RULE,
                    "missingClosingSpace",
                    "There must be a space before this paren.",
                    token.start,
                );
            } else if !always && is_whitespace(gap) {
                report_unexpected_space(
                    diagnostics,
                    RULE,
                    "rejectedClosingSpace",
                    "There should be no space before this paren.",
                    prev.end,
                    token.start,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// template-curly-spacing
// ---------------------------------------------------------------------------

pub(crate) fn check_template_curly_spacing(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "template-curly-spacing";
    let always = option_keyword(options, "never") == "always";

    for index in 0..scan.tokens().len() {
        let token = &scan.tokens()[index];
        match token.kind {
            // `${` lives at the end of a head/middle chunk: check the gap after.
            TokenKind::TemplateHead | TokenKind::TemplateMiddle => {
                let Some(next) = scan.tokens().get(index + 1) else {
                    continue;
                };
                let gap = scan.gap(token, next);
                if has_newline(gap) {
                    continue;
                }
                if always && gap.is_empty() {
                    report_missing_space(
                        diagnostics,
                        RULE,
                        "expectedAfter",
                        "Expected space(s) after '${'.",
                        token.end,
                    );
                } else if !always && is_whitespace(gap) {
                    report_unexpected_space(
                        diagnostics,
                        RULE,
                        "unexpectedAfter",
                        "Unexpected space(s) after '${'.",
                        token.end,
                        next.start,
                    );
                }
            }
            _ => {}
        }
        // `}` lives at the start of a middle/tail chunk: check the gap before.
        if matches!(
            token.kind,
            TokenKind::TemplateMiddle | TokenKind::TemplateTail
        ) && index > 0
        {
            let Some(prev) = scan.tokens().get(index - 1) else {
                continue;
            };
            let gap = scan.gap(prev, token);
            if has_newline(gap) {
                continue;
            }
            if always && gap.is_empty() {
                report_missing_space(
                    diagnostics,
                    RULE,
                    "expectedBefore",
                    "Expected space(s) before '}'.",
                    token.start,
                );
            } else if !always && is_whitespace(gap) {
                report_unexpected_space(
                    diagnostics,
                    RULE,
                    "unexpectedBefore",
                    "Unexpected space(s) before '}'.",
                    prev.end,
                    token.start,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rest-spread-spacing
// ---------------------------------------------------------------------------

pub(crate) fn check_rest_spread_spacing(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "rest-spread-spacing";
    let always = option_keyword(options, "never") == "always";

    for index in 0..scan.tokens().len() {
        let token = &scan.tokens()[index];
        if !punct_is(token, scan.source(), "...") {
            continue;
        }
        let Some(next) = scan.tokens().get(index + 1) else {
            continue;
        };
        let gap = scan.gap(token, next);
        if has_newline(gap) {
            continue;
        }
        if always && gap.is_empty() {
            report_missing_space(
                diagnostics,
                RULE,
                "expectedWhitespace",
                "Expected whitespace after spread operator.",
                token.end,
            );
        } else if !always && is_whitespace(gap) {
            report_unexpected_space(
                diagnostics,
                RULE,
                "unexpectedWhitespace",
                "Unexpected whitespace after spread operator.",
                token.end,
                next.start,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// no-multi-spaces
// ---------------------------------------------------------------------------

pub(crate) fn check_no_multi_spaces(
    scan: &Scan,
    _options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "no-multi-spaces";
    for window in scan.tokens().windows(2) {
        let [a, b] = window else { continue };
        let gap = scan.gap(a, b);
        if gap.len() < 2 || has_newline(gap) {
            continue;
        }
        if gap.bytes().all(|byte| byte == b' ' || byte == b'\t') {
            report_replace(
                diagnostics,
                RULE,
                "multipleSpaces",
                "Multiple spaces found.",
                a.end,
                b.start,
                "collapseSpace",
                "Collapse to a single space.",
                " ",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// no-whitespace-before-property
// ---------------------------------------------------------------------------

pub(crate) fn check_no_whitespace_before_property(
    scan: &Scan,
    _options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "no-whitespace-before-property";
    for index in 1..scan.tokens().len() {
        let dot = &scan.tokens()[index];
        if !punct_is(dot, scan.source(), ".") && !punct_is(dot, scan.source(), "?.") {
            continue;
        }
        let Some(prev) = scan.tokens().get(index - 1) else {
            continue;
        };
        // Only flag a member access after an expression-ending token.
        if !matches!(
            prev.kind,
            TokenKind::Identifier
                | TokenKind::Number
                | TokenKind::String
                | TokenKind::NoSubTemplate
                | TokenKind::TemplateTail
        ) && !punct_is(prev, scan.source(), ")")
            && !punct_is(prev, scan.source(), "]")
        {
            continue;
        }
        // Whitespace on *either* side of the dot separates the object from its
        // property; `@stylistic` reports the member access once for either.
        let before = scan.gap(prev, dot);
        let before_ws = is_whitespace(before) && !has_newline(before);
        let after_ws = scan
            .tokens()
            .get(index + 1)
            .map(|next| {
                let gap = scan.gap(dot, next);
                is_whitespace(gap) && !has_newline(gap)
            })
            .unwrap_or(false);
        if before_ws || after_ws {
            let (start, end) = if before_ws {
                (prev.end, dot.start)
            } else {
                (dot.end, scan.tokens()[index + 1].start)
            };
            report_unexpected_space(
                diagnostics,
                RULE,
                "unexpectedWhitespace",
                "Unexpected whitespace before property.",
                start,
                end,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// dot-location
// ---------------------------------------------------------------------------

pub(crate) fn check_dot_location(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "dot-location";
    let on_property = option_keyword(options, "object") == "property";

    for index in 0..scan.tokens().len() {
        let dot = &scan.tokens()[index];
        if !punct_is(dot, scan.source(), ".") {
            continue;
        }
        let prev = (index > 0).then(|| scan.tokens().get(index - 1)).flatten();
        let next = scan.tokens().get(index + 1);
        if on_property {
            // Dot must sit with the property: no newline between dot and property.
            if let Some(next) = next {
                if has_newline(scan.gap(dot, next)) {
                    report_replace(
                        diagnostics,
                        RULE,
                        "expectedDotBeforeProperty",
                        "Expected dot to be on same line as property.",
                        dot.start,
                        dot.end,
                        "moveDot",
                        "Move the dot to the property.",
                        "",
                    );
                }
            }
        } else {
            // Dot must sit with the object: no newline between object and dot.
            if let Some(prev) = prev {
                if has_newline(scan.gap(prev, dot)) {
                    report_replace(
                        diagnostics,
                        RULE,
                        "expectedDotAfterObject",
                        "Expected dot to be on same line as object.",
                        dot.start,
                        dot.end,
                        "moveDot",
                        "Move the dot to the object.",
                        "",
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// spaced-comment
// ---------------------------------------------------------------------------

pub(crate) fn check_spaced_comment(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "spaced-comment";
    let always = option_keyword(options, "always") != "never";

    for token in scan.tokens() {
        match token.kind {
            TokenKind::LineComment => {
                let text = scan.slice(token.start, token.end);
                let body = &text[2..]; // after `//`
                // Triple-slash directives and empty comments are exempt.
                if body.starts_with('/') || body.is_empty() {
                    continue;
                }
                let has_space = body.starts_with([' ', '\t']);
                // `!` is a conventional marker (e.g. license banners).
                let is_marker = body.starts_with('!');
                if always && !has_space && !is_marker {
                    report_missing_space(
                        diagnostics,
                        RULE,
                        "expectedSpaceAfter",
                        "Expected space after '//'.",
                        token.start + 2,
                    );
                } else if !always && has_space {
                    let space_len = body.len() - body.trim_start_matches([' ', '\t']).len();
                    report_unexpected_space(
                        diagnostics,
                        RULE,
                        "unexpectedSpaceAfter",
                        "Unexpected space after '//'.",
                        token.start + 2,
                        token.start + 2 + space_len,
                    );
                }
            }
            TokenKind::BlockComment => {
                let text = scan.slice(token.start, token.end);
                if text.len() < 4 {
                    continue;
                }
                let inner = &text[2..text.len() - 2]; // between `/*` and `*/`
                if inner.starts_with('*') || inner.is_empty() {
                    continue; // JSDoc `/**` and empty `/**/` are exempt.
                }
                let has_space = inner.starts_with([' ', '\t', '\n', '\r']);
                let is_marker = inner.starts_with('!');
                if always && !has_space && !is_marker {
                    report_missing_space(
                        diagnostics,
                        RULE,
                        "expectedSpaceAfter",
                        "Expected space after '/*'.",
                        token.start + 2,
                    );
                } else if !always && has_space {
                    let space_len = inner.len() - inner.trim_start_matches([' ', '\t']).len();
                    if space_len > 0 {
                        report_unexpected_space(
                            diagnostics,
                            RULE,
                            "unexpectedSpaceAfter",
                            "Unexpected space after '/*'.",
                            token.start + 2,
                            token.start + 2 + space_len,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// line-comment-position
// ---------------------------------------------------------------------------

pub(crate) fn check_line_comment_position(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const RULE: &str = "line-comment-position";
    let config = LineCommentPositionConfig::from_options(options);

    for (index, token) in scan.tokens().iter().enumerate() {
        if token.kind != TokenKind::LineComment {
            continue;
        }

        let comment_value = &scan.source()[token.start + 2..token.end];
        if config.should_ignore(comment_value) {
            continue;
        }

        let is_on_same_line = index
            .checked_sub(1)
            .and_then(|previous_index| scan.tokens().get(previous_index))
            .is_some_and(|previous| !has_newline(scan.gap(previous, token)));

        match config.position {
            LineCommentPosition::Above if is_on_same_line => push_diagnostic(
                diagnostics,
                RULE,
                "above",
                "Expected comment to be above code.",
                token.start,
                token.end,
                None::<(
                    &'static str,
                    &'static str,
                    fn(crate::TextRange) -> crate::LintFix,
                )>,
            ),
            LineCommentPosition::Beside if !is_on_same_line => push_diagnostic(
                diagnostics,
                RULE,
                "beside",
                "Expected comment to be beside code.",
                token.start,
                token.end,
                None::<(
                    &'static str,
                    &'static str,
                    fn(crate::TextRange) -> crate::LintFix,
                )>,
            ),
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
enum LineCommentPosition {
    Above,
    Beside,
}

struct LineCommentPositionConfig {
    position: LineCommentPosition,
    apply_default_ignore_patterns: bool,
    custom_ignore: Option<regex::Regex>,
}

impl LineCommentPositionConfig {
    fn from_options(options: &Value) -> Self {
        let option = first_option(options);
        let mut position = LineCommentPosition::Above;
        let mut ignore_pattern = None;
        let mut apply_default_patterns = true;
        let mut apply_default_ignore_patterns = None;

        match option {
            Some(Value::String(value)) if value == "beside" => {
                position = LineCommentPosition::Beside;
            }
            Some(Value::Object(object)) => {
                if object
                    .get("position")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value == "beside")
                {
                    position = LineCommentPosition::Beside;
                }
                ignore_pattern = object.get("ignorePattern").and_then(Value::as_str);
                if let Some(value) = object.get("applyDefaultPatterns").and_then(Value::as_bool) {
                    apply_default_patterns = value;
                }
                apply_default_ignore_patterns = object
                    .get("applyDefaultIgnorePatterns")
                    .and_then(Value::as_bool);
            }
            _ => {}
        }

        let custom_ignore = ignore_pattern.and_then(|pattern| regex::Regex::new(pattern).ok());
        Self {
            position,
            apply_default_ignore_patterns: apply_default_ignore_patterns
                .unwrap_or(apply_default_patterns),
            custom_ignore,
        }
    }

    fn should_ignore(&self, comment_value: &str) -> bool {
        (self.apply_default_ignore_patterns && is_default_ignored_line_comment(comment_value))
            || self
                .custom_ignore
                .as_ref()
                .is_some_and(|regex| regex.is_match(comment_value))
    }
}

fn first_option(options: &Value) -> Option<&Value> {
    match options {
        Value::Array(items) => items.first(),
        Value::Null => None,
        other => Some(other),
    }
}

fn is_default_ignored_line_comment(comment_value: &str) -> bool {
    let trimmed = comment_value.trim_start_matches([' ', '\t', '\u{000b}', '\u{000c}']);
    is_comments_ignore_pattern(trimmed) || is_fallthrough_comment(trimmed)
}

fn is_comments_ignore_pattern(trimmed: &str) -> bool {
    trimmed.starts_with("eslint")
        || trimmed.starts_with("jscs")
        || starts_with_word_and_space(trimmed, "jshint")
        || starts_with_word_and_space(trimmed, "jslint")
        || starts_with_word_and_space(trimmed, "istanbul")
        || starts_with_word_and_space(trimmed, "global")
        || starts_with_word_and_space(trimmed, "globals")
        || starts_with_word_and_space(trimmed, "exported")
}

fn starts_with_word_and_space(text: &str, word: &str) -> bool {
    text.strip_prefix(word)
        .and_then(|rest| rest.as_bytes().first().copied())
        .is_some_and(is_ascii_space)
}

fn is_ascii_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | 0x0b | 0x0c)
}

fn is_fallthrough_comment(trimmed: &str) -> bool {
    trimmed.starts_with("fallthrough")
        || trimmed.starts_with("fall through")
        || trimmed.starts_with("fallsthrough")
        || trimmed.starts_with("falls through")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(
        check: fn(&Scan, &Value, &mut Vec<LintDiagnostic>),
        source: &str,
        options: Value,
    ) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check(&scan, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(diagnostics: &[LintDiagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.message_id.as_str()).collect()
    }

    fn object_options<const N: usize>(entries: [(&str, Value); N]) -> Value {
        Value::Array(
            std::iter::once(Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), value))
                    .collect(),
            ))
            .collect(),
        )
    }

    #[test]
    fn arrow_spacing_flags_missing_and_unexpected() {
        assert!(run(check_arrow_spacing, "const f = ()=>1;", Value::Null).len() == 2);
        assert!(run(check_arrow_spacing, "const f = () => 1;", Value::Null).is_empty());
    }

    #[test]
    fn comma_spacing_defaults() {
        assert_eq!(
            ids(&run(check_comma_spacing, "[1 ,2]", Value::Null)),
            ["unexpected", "missing"]
        );
        assert!(run(check_comma_spacing, "[1, 2]", Value::Null).is_empty());
        // Trailing comma before close bracket is exempt for the after-check.
        assert!(run(check_comma_spacing, "[1, 2,]", Value::Null).is_empty());
    }

    #[test]
    fn semi_spacing_defaults() {
        assert_eq!(
            ids(&run(check_semi_spacing, "a ;b", Value::Null)),
            ["unexpected", "missing"]
        );
        assert!(run(check_semi_spacing, "a; b", Value::Null).is_empty());
        assert!(run(check_semi_spacing, "for (;;) {}", Value::Null).is_empty());
    }

    #[test]
    fn space_in_parens_never_and_always() {
        assert_eq!(
            ids(&run(check_space_in_parens, "f( a )", Value::Null)).len(),
            2
        );
        assert!(run(check_space_in_parens, "f(a)", Value::Null).is_empty());
        assert!(run(check_space_in_parens, "f()", Value::Null).is_empty());
        let always = Value::Array(std::iter::once(Value::String("always".into())).collect());
        assert_eq!(run(check_space_in_parens, "f(a)", always).len(), 2);
    }

    #[test]
    fn template_curly_spacing_never() {
        assert_eq!(
            ids(&run(check_template_curly_spacing, "`${ x }`", Value::Null)).len(),
            2
        );
        assert!(run(check_template_curly_spacing, "`${x}`", Value::Null).is_empty());
    }

    #[test]
    fn rest_spread_spacing_never() {
        assert_eq!(
            ids(&run(check_rest_spread_spacing, "f(... args)", Value::Null)),
            ["unexpectedWhitespace"]
        );
        assert!(run(check_rest_spread_spacing, "f(...args)", Value::Null).is_empty());
    }

    #[test]
    fn no_multi_spaces_collapses() {
        assert_eq!(
            ids(&run(check_no_multi_spaces, "a  =  b", Value::Null)),
            ["multipleSpaces", "multipleSpaces"]
        );
        assert!(run(check_no_multi_spaces, "a = b", Value::Null).is_empty());
        // Leading indentation is preceded by a newline, so it is never flagged.
        assert!(run(check_no_multi_spaces, "a\n    b", Value::Null).is_empty());
    }

    #[test]
    fn no_whitespace_before_property_flags_space() {
        assert_eq!(
            ids(&run(
                check_no_whitespace_before_property,
                "foo .bar",
                Value::Null
            )),
            ["unexpectedWhitespace"]
        );
        assert!(run(check_no_whitespace_before_property, "foo.bar", Value::Null).is_empty());
        // Newline before the dot is left to dot-location, not this rule.
        assert!(
            run(
                check_no_whitespace_before_property,
                "foo\n.bar",
                Value::Null
            )
            .is_empty()
        );
    }

    #[test]
    fn dot_location_object_default() {
        assert_eq!(
            ids(&run(check_dot_location, "foo\n.bar", Value::Null)),
            ["expectedDotAfterObject"]
        );
        assert!(run(check_dot_location, "foo.\nbar", Value::Null).is_empty());
        let property = Value::Array(std::iter::once(Value::String("property".into())).collect());
        assert_eq!(
            ids(&run(check_dot_location, "foo.\nbar", property)),
            ["expectedDotBeforeProperty"]
        );
    }

    #[test]
    fn spaced_comment_always() {
        assert_eq!(
            ids(&run(check_spaced_comment, "//x", Value::Null)),
            ["expectedSpaceAfter"]
        );
        assert!(run(check_spaced_comment, "// x", Value::Null).is_empty());
        assert!(run(check_spaced_comment, "/// <reference />", Value::Null).is_empty());
        assert_eq!(
            ids(&run(check_spaced_comment, "/*x*/", Value::Null)),
            ["expectedSpaceAfter"]
        );
        assert!(run(check_spaced_comment, "/** jsdoc */", Value::Null).is_empty());
    }

    #[test]
    fn line_comment_position_defaults_to_above() {
        assert_eq!(
            ids(&run(
                check_line_comment_position,
                "const value = 1; // inline\n// above\nvalue;\n",
                Value::Null
            )),
            ["above"]
        );
        assert_eq!(
            run(
                check_line_comment_position,
                "const value = 1; // inline\n",
                Value::Null
            )[0]
            .range
            .start,
            17
        );
    }

    #[test]
    fn line_comment_position_beside_reports_above_comments() {
        let options = Value::Array(std::iter::once(Value::String("beside".into())).collect());
        assert_eq!(
            ids(&run(
                check_line_comment_position,
                "// above\nvalue; // beside\n",
                options
            )),
            ["beside"]
        );
    }

    #[test]
    fn line_comment_position_honors_default_ignores() {
        assert!(
            run(
                check_line_comment_position,
                "value; // eslint-disable-line\nvalue; // global NAME\nvalue; // globals NAME: true\nvalue; // jshint ignore:line\nvalue; // jslint vars: true\nvalue; // istanbul ignore next\nvalue; // jscs: disable\nvalue; // exported NAME\nvalue; // fallthrough\nvalue; // fall through\nvalue; // falls through\n",
                Value::Null
            )
            .is_empty()
        );
        assert_eq!(
            ids(&run(
                check_line_comment_position,
                "value; // globalization is a word\nvalue; // mentioning falls through\n",
                Value::Null
            )),
            ["above", "above"]
        );
    }

    #[test]
    fn line_comment_position_honors_custom_and_deprecated_ignore_options() {
        let custom = object_options([
            ("position", Value::String("above".into())),
            ("ignorePattern", Value::String("linter|pragma".into())),
        ]);
        assert_eq!(
            ids(&run(
                check_line_comment_position,
                "value; // linter\nvalue; // invalid\n",
                custom
            )),
            ["above"]
        );

        let disabled_default = object_options([
            ("position", Value::String("beside".into())),
            ("applyDefaultPatterns", Value::Bool(false)),
        ]);
        assert_eq!(
            ids(&run(
                check_line_comment_position,
                "// jscs: disable\nvalue;\n",
                disabled_default
            )),
            ["beside"]
        );

        let new_option_takes_precedence = object_options([
            ("position", Value::String("beside".into())),
            ("applyDefaultPatterns", Value::Bool(true)),
            ("applyDefaultIgnorePatterns", Value::Bool(false)),
        ]);
        assert_eq!(
            ids(&run(
                check_line_comment_position,
                "// jscs: disable\nvalue;\n",
                new_option_takes_precedence
            )),
            ["beside"]
        );
    }
}
