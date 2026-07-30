//! Native source-wide fallback for `@stylistic/indent`.
//!
//! The Oxlint plugin executes the pinned upstream visitor directly so it can
//! preserve the rule's full ESTree/TypeScript/JSX offset graph. The native API
//! still exposes deterministic indentation diagnostics for source-wide callers:
//! this scanner covers delimiter nesting, top-level indentation, comments,
//! spaces/tabs, CRLF, and malformed input without requiring an AST bridge.

use std::{collections::BTreeMap, fmt::Write as _};

use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::{context::Scan, lexer::TokenKind};

const RULE_NAME: &str = "indent";
const MESSAGE_ID: &str = "wrongIndentation";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IndentStyle {
    Spaces(usize),
    Tabs,
}

impl IndentStyle {
    fn from_options(options: &Value) -> Self {
        let first = match options {
            Value::Array(items) => items.first(),
            Value::Null => None,
            value => Some(value),
        };
        match first {
            Some(Value::String(value)) if value == "tab" => Self::Tabs,
            Some(Value::Number(value)) => value
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .map_or(Self::Spaces(4), Self::Spaces),
            _ => Self::Spaces(4),
        }
    }

    fn indent(self, depth: usize) -> String {
        match self {
            Self::Spaces(width) => " ".repeat(width.saturating_mul(depth)),
            Self::Tabs => "\t".repeat(depth),
        }
    }

    fn expected_label(self, expected: &str) -> String {
        let amount = expected.chars().count();
        let unit = match self {
            Self::Spaces(_) => "space",
            Self::Tabs => "tab",
        };
        count_with_unit(amount, unit)
    }

    fn actual_label(self, actual: &str) -> String {
        let spaces = actual.bytes().filter(|byte| *byte == b' ').count();
        let tabs = actual.bytes().filter(|byte| *byte == b'\t').count();
        match self {
            Self::Spaces(_) if tabs == 0 => decimal_string(spaces),
            Self::Tabs if spaces == 0 => decimal_string(tabs),
            _ if spaces > 0 => count_with_unit(spaces, "space"),
            _ if tabs > 0 => count_with_unit(tabs, "tab"),
            _ => "0".to_owned(),
        }
    }
}

fn count_with_unit(amount: usize, unit: &str) -> String {
    let mut output = String::with_capacity(24);
    let _ = write!(output, "{amount}");
    output.push(' ');
    output.push_str(unit);
    if amount != 1 {
        output.push('s');
    }
    output
}

fn decimal_string(value: usize) -> String {
    let mut output = String::with_capacity(20);
    let _ = write!(output, "{value}");
    output
}

#[derive(Clone, Copy)]
struct Options {
    style: IndentStyle,
    ignore_comments: bool,
    switch_case: usize,
}

impl Options {
    fn from_value(value: &Value) -> Self {
        let style = IndentStyle::from_options(value);
        let object = match value {
            Value::Array(items) => items.get(1).and_then(Value::as_object),
            _ => None,
        };
        let ignore_comments = object
            .and_then(|options| options.get("ignoreComments"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let switch_case = object
            .and_then(|options| options.get("SwitchCase"))
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        Self {
            style,
            ignore_comments,
            switch_case,
        }
    }
}

#[derive(Clone, Copy)]
struct Line {
    start: usize,
    content_end: usize,
}

pub(crate) fn check_indent(
    scan: &Scan<'_>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_value(options);
    let lines = collect_lines(scan.source());
    let mut token_index = 0;
    let mut delimiters: Vec<&str> = Vec::new();

    for line in lines {
        while token_index < scan.tokens().len() && scan.tokens()[token_index].end <= line.start {
            update_delimiters(scan, token_index, &mut delimiters);
            token_index += 1;
        }

        let first_index = (token_index..scan.tokens().len()).find(|index| {
            let token = scan.tokens()[*index];
            token.start >= line.start && token.start < line.content_end
        });
        let Some(first_index) = first_index else {
            continue;
        };
        let first = scan.tokens()[first_index];
        let text = scan.token_text(first_index);
        let actual = &scan.source()[line.start..first.start];
        if !actual.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
            continue;
        }

        let closes_delimiter = matches!(text, "}" | "]" | ")");
        let is_switch_label = matches!(text, "case" | "default");
        let mut depth = delimiters
            .len()
            .saturating_sub(usize::from(closes_delimiter));
        if is_switch_label {
            depth = depth.saturating_sub(1).saturating_add(options.switch_case);
        }
        let expected = options.style.indent(depth);
        if actual != expected && !(options.ignore_comments && first.kind.is_comment()) {
            report(
                line.start,
                first.start,
                actual,
                &expected,
                options.style,
                diagnostics,
            );
        }

        while token_index < scan.tokens().len()
            && scan.tokens()[token_index].start < line.content_end
        {
            update_delimiters(scan, token_index, &mut delimiters);
            token_index += 1;
        }
    }
}

fn update_delimiters<'source>(
    scan: &'source Scan<'source>,
    index: usize,
    delimiters: &mut Vec<&'source str>,
) {
    let token = scan.tokens()[index];
    if token.kind != TokenKind::Punctuator {
        return;
    }
    let text = scan.token_text(index);
    match text {
        "{" | "[" | "(" => delimiters.push(text),
        "}" => pop_matching(delimiters, "{"),
        "]" => pop_matching(delimiters, "["),
        ")" => pop_matching(delimiters, "("),
        _ => {}
    }
}

fn pop_matching(delimiters: &mut Vec<&str>, expected: &str) {
    if delimiters
        .last()
        .is_some_and(|delimiter| *delimiter == expected)
    {
        delimiters.pop();
    }
}

fn report(
    start: usize,
    end: usize,
    actual: &str,
    expected: &str,
    style: IndentStyle,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
        return;
    };
    let range = TextRange::new(start, end);
    let expected_label = style.expected_label(expected);
    let actual_label = style.actual_label(actual);
    let mut message =
        String::with_capacity(43 + expected_label.len().saturating_add(actual_label.len()));
    message.push_str("Expected indentation of ");
    message.push_str(&expected_label);
    message.push_str(" but found ");
    message.push_str(&actual_label);
    message.push('.');
    diagnostics.push(LintDiagnostic {
        rule_name: RULE_NAME.to_owned(),
        message_id: MESSAGE_ID.to_owned(),
        message: message.clone(),
        data: BTreeMap::from([
            ("expected".to_owned(), expected_label),
            ("actual".to_owned(), actual_label),
        ]),
        range,
        suggestions: std::iter::once(LintSuggestion {
            message_id: MESSAGE_ID.to_owned(),
            message,
            fixes: std::iter::once(LintFix::replace_range(range, expected)).collect(),
        })
        .collect(),
    });
}

fn collect_lines(source: &str) -> Vec<Line> {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let mut cursor = start;
        while cursor < bytes.len() {
            if matches!(bytes[cursor], b'\n' | b'\r') {
                break;
            }
            if bytes[cursor..].starts_with(&[0xe2, 0x80, 0xa8])
                || bytes[cursor..].starts_with(&[0xe2, 0x80, 0xa9])
            {
                break;
            }
            cursor += 1;
        }
        lines.push(Line {
            start,
            content_end: cursor,
        });
        if cursor >= bytes.len() {
            break;
        }
        start = if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
            cursor + 2
        } else if bytes[cursor] == 0xe2 {
            cursor + 3
        } else {
            cursor + 1
        };
    }
    lines
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the small test-only option matrices readable"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check_indent(&scan, &options, &mut diagnostics);
        diagnostics
    }

    #[test]
    fn reports_nested_spaces_with_exact_data_and_fix() {
        let source = "function value() {\nreturn {\nanswer: 42,\n};\n}\n";
        let diagnostics = run(source, json!([2]));
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(diagnostics[0].range, TextRange::new(19, 19));
        assert_eq!(diagnostics[0].data["expected"], "2 spaces");
        assert_eq!(diagnostics[0].data["actual"], "0");
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0].replacement_text,
            "  "
        );
    }

    #[test]
    fn supports_tabs_crlf_comments_and_switch_case_options() {
        let source = "switch (value) {\r\ncase 1:\r\n// note\r\nuse();\r\n}\r\n";
        let diagnostics = run(
            source,
            json!(["tab", { "SwitchCase": 1, "ignoreComments": true }]),
        );
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.data["expected"].ends_with("tab"))
        );
    }

    #[test]
    fn tolerates_unicode_line_terminators_and_malformed_delimiters() {
        let diagnostics = run("if (ready) {\u{2028}value();\u{2029}}\n]\n", json!([2]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].data["expected"], "2 spaces");
    }
}
