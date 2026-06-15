use serde_json::Value;

use crate::{LintDiagnostic, LintFix, TextRange};

use super::{
    helpers::{
        ReplacementDiagnostic, option_bool, option_str, option_usize, push_diagnostic,
        push_replacement_diagnostic,
    },
    lexer::{Token, TokenKind, tokenize},
    line_index::{LineInfo, Newline},
};

/// `max-len`: report lines whose display width exceeds the configured maximum.
///
/// Implements the most-used options: `code`, `tabWidth`, `comments`,
/// `ignoreComments`, and `ignoreUrls`. The positional legacy forms
/// `[code]` and `[code, tabWidth]` are accepted alongside the object form.
pub(crate) fn check_max_len(
    source_text: &str,
    lines: &[LineInfo],
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let code = max_len_number(options, 0, "code").unwrap_or(80);
    let tab_width = max_len_number(options, 1, "tabWidth").unwrap_or(4).max(1);
    let comment_limit = max_len_object(options)
        .and_then(|object| object.get("comments"))
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    let ignore_comments = max_len_flag(options, "ignoreComments");
    let ignore_urls = max_len_flag(options, "ignoreUrls");

    let bytes = source_text.as_bytes();
    for line in lines {
        let text = &source_text[line.start..line.content_end];
        if ignore_comments && line.is_comment_only {
            continue;
        }
        if ignore_urls && text.contains("://") {
            continue;
        }
        let limit = if line.is_comment_only {
            comment_limit.unwrap_or(code)
        } else {
            code
        };
        let width = display_width(bytes, line.start, line.content_end, tab_width);
        if width > limit {
            let (message_id, message): (&str, &str) = if line.is_comment_only {
                (
                    "tooLongComment",
                    "This comment line exceeds the maximum allowed length.",
                )
            } else {
                ("tooLong", "This line exceeds the maximum allowed length.")
            };
            diagnostics.push(LintDiagnostic {
                rule_name: "max-len".to_owned(),
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                range: TextRange::new(line.start as u32, line.content_end as u32),
                suggestions: Vec::new(),
            });
        }
    }
}

/// Counts the display columns of `[start, end)`, expanding tabs to `tab_width`
/// columns each and counting every other UTF-8 scalar as one column.
fn display_width(bytes: &[u8], start: usize, end: usize, tab_width: usize) -> usize {
    let mut width = 0;
    let mut index = start;
    while index < end {
        let byte = bytes[index];
        if byte == b'\t' {
            width += tab_width;
            index += 1;
        } else {
            // Advance one UTF-8 scalar; continuation bytes are 0b10xxxxxx.
            width += 1;
            index += 1;
            while index < end && (bytes[index] & 0xC0) == 0x80 {
                index += 1;
            }
        }
    }
    width
}

fn max_len_object(options: &Value) -> Option<&serde_json::Map<String, Value>> {
    match options {
        Value::Array(items) => items.iter().find_map(Value::as_object),
        Value::Object(object) => Some(object),
        _ => None,
    }
}

/// Reads a `max-len` number from either the positional slot or the object form.
fn max_len_number(options: &Value, position: usize, key: &str) -> Option<usize> {
    if let Value::Array(items) = options {
        if let Some(value) = items.get(position).and_then(Value::as_u64) {
            return Some(value as usize);
        }
    }
    max_len_object(options)
        .and_then(|object| object.get(key))
        .and_then(Value::as_u64)
        .map(|value| value as usize)
}

fn max_len_flag(options: &Value, key: &str) -> bool {
    max_len_object(options)
        .and_then(|object| object.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn check_no_trailing_spaces(
    source_text: &str,
    lines: &[LineInfo],
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let skip_blank_lines = option_bool(options, 0, "skipBlankLines", false);
    let ignore_comments = option_bool(options, 0, "ignoreComments", false);

    for line in lines {
        if (skip_blank_lines && line.is_blank) || (ignore_comments && line.is_comment_only) {
            continue;
        }
        let trailing_start =
            trim_ascii_space_end(source_text.as_bytes(), line.start, line.content_end);
        if trailing_start < line.content_end {
            push_diagnostic(
                diagnostics,
                "no-trailing-spaces",
                "trailingSpace",
                "Trailing spaces are not allowed.",
                trailing_start,
                line.content_end,
                Some((
                    "removeTrailingSpace",
                    "Remove trailing spaces.",
                    LintFix::remove_range,
                )),
            );
        }
    }
}

pub(crate) fn check_no_mixed_spaces_and_tabs(
    source_text: &str,
    lines: &[LineInfo],
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let smart_tabs = smart_tabs_enabled(options);
    let tokens = tokenize(source_text);

    for (line_index, line) in lines.iter().enumerate() {
        let Some((start, end)) = mixed_indent_range(source_text.as_bytes(), line, smart_tabs)
        else {
            continue;
        };
        if is_ignored_mixed_indent_position(&tokens, lines, line_index, start) {
            continue;
        }
        push_diagnostic(
            diagnostics,
            "no-mixed-spaces-and-tabs",
            "mixedSpacesAndTabs",
            "Mixed spaces and tabs.",
            start,
            end,
            None::<(
                &'static str,
                &'static str,
                fn(crate::TextRange) -> crate::LintFix,
            )>,
        );
    }
}

pub(crate) fn check_eol_last(
    source_text: &str,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if source_text.is_empty() {
        return;
    }

    match option_str(options, 0).unwrap_or("always") {
        "never" => check_no_eol_last(source_text, diagnostics),
        _ if !source_text.ends_with('\n') && !source_text.ends_with('\r') => {
            push_replacement_diagnostic(
                diagnostics,
                ReplacementDiagnostic {
                    rule_name: "eol-last",
                    message_id: "missing",
                    message: "Expected newline at end of file.",
                    start: source_text.len(),
                    end: source_text.len(),
                    suggestion_id: "insertNewline",
                    suggestion_message: "Insert a newline.",
                },
                "\n",
            );
        }
        _ => {}
    }
}

pub(crate) fn check_linebreak_style(
    lines: &[LineInfo],
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let expected = option_str(options, 0).unwrap_or("unix");
    for line in lines {
        if line.newline == Newline::None {
            continue;
        }
        match expected {
            "windows" if line.newline != Newline::Crlf => report_linebreak(
                diagnostics,
                line,
                "expectedWindows",
                "Expected Windows linebreaks.",
                "\r\n",
            ),
            _ if expected != "windows" && line.newline != Newline::Lf => report_linebreak(
                diagnostics,
                line,
                "expectedUnix",
                "Expected Unix linebreaks.",
                "\n",
            ),
            _ => {}
        }
    }
}

pub(crate) fn check_no_multiple_empty_lines(
    lines: &[LineInfo],
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let max = option_usize(options, 0, "max", 2);
    let max_bof = option_usize(options, 0, "maxBOF", max);
    let max_eof = option_usize(options, 0, "maxEOF", max);
    let Some(first_non_blank) = lines.iter().position(|line| !line.is_blank) else {
        report_extra_blank_lines(lines, max_bof.min(max_eof), diagnostics);
        return;
    };
    let last_non_blank = lines
        .iter()
        .rposition(|line| !line.is_blank)
        .unwrap_or(first_non_blank);

    report_extra_blank_lines(&lines[..first_non_blank], max_bof, diagnostics);
    report_extra_blank_lines(&lines[last_non_blank + 1..], max_eof, diagnostics);
    report_middle_blank_runs(lines, first_non_blank, last_non_blank, max, diagnostics);
}

fn check_no_eol_last(source_text: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    let trim_start = source_text
        .as_bytes()
        .iter()
        .rposition(|byte| *byte != b'\n' && *byte != b'\r')
        .map_or(0, |index| index + 1);
    if trim_start < source_text.len() {
        push_diagnostic(
            diagnostics,
            "eol-last",
            "unexpected",
            "Unexpected newline at end of file.",
            trim_start,
            source_text.len(),
            Some((
                "removeNewline",
                "Remove trailing newlines.",
                LintFix::remove_range,
            )),
        );
    }
}

fn report_linebreak(
    diagnostics: &mut Vec<LintDiagnostic>,
    line: &LineInfo,
    message_id: &'static str,
    message: &'static str,
    replacement: &'static str,
) {
    push_replacement_diagnostic(
        diagnostics,
        ReplacementDiagnostic {
            rule_name: "linebreak-style",
            message_id,
            message,
            start: line.newline_start,
            end: line.newline_start + line.newline_len,
            suggestion_id: "fixLinebreak",
            suggestion_message: "Replace linebreak.",
        },
        replacement,
    );
}

fn report_middle_blank_runs(
    lines: &[LineInfo],
    first_non_blank: usize,
    last_non_blank: usize,
    max: usize,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let mut run_start = None;
    for (index, line) in lines
        .iter()
        .enumerate()
        .take(last_non_blank)
        .skip(first_non_blank + 1)
    {
        if line.is_blank {
            run_start.get_or_insert(index);
        } else if let Some(start) = run_start.take() {
            report_extra_blank_lines(&lines[start..index], max, diagnostics);
        }
    }
    if let Some(start) = run_start {
        report_extra_blank_lines(&lines[start..last_non_blank], max, diagnostics);
    }
}

fn report_extra_blank_lines(
    blank_lines: &[LineInfo],
    allowed: usize,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if blank_lines.len() <= allowed {
        return;
    }
    // `@stylistic` reports one violation for the whole excess run, not one per
    // blank line. The fix removes the surplus lines.
    let excess = &blank_lines[allowed..];
    let start = excess[0].start;
    let end = excess[excess.len() - 1].end;
    push_diagnostic(
        diagnostics,
        "no-multiple-empty-lines",
        "tooMany",
        "Too many blank lines.",
        start,
        end,
        Some((
            "removeBlankLine",
            "Remove extra blank line.",
            LintFix::remove_range,
        )),
    );
}

fn trim_ascii_space_end(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut cursor = end;
    while cursor > start {
        let byte = bytes[cursor - 1];
        if byte != b' ' && byte != b'\t' {
            return cursor;
        }
        cursor -= 1;
    }
    start
}

fn smart_tabs_enabled(options: &Value) -> bool {
    match options {
        Value::Array(items) => items.first().is_some_and(|value| {
            value.as_bool().unwrap_or(false) || value.as_str() == Some("smart-tabs")
        }),
        Value::Bool(value) => *value,
        Value::String(value) => value == "smart-tabs",
        _ => false,
    }
}

fn mixed_indent_range(bytes: &[u8], line: &LineInfo, smart_tabs: bool) -> Option<(usize, usize)> {
    if smart_tabs {
        smart_tabs_mixed_indent_range(bytes, line)
    } else {
        default_mixed_indent_range(bytes, line)
    }
}

fn default_mixed_indent_range(bytes: &[u8], line: &LineInfo) -> Option<(usize, usize)> {
    let first = *bytes.get(line.start)?;
    if !matches!(first, b' ' | b'\t') {
        return None;
    }

    let mut cursor = line.start + 1;
    while cursor < line.content_end && bytes[cursor] == first {
        cursor += 1;
    }

    if cursor < line.content_end
        && ((first == b' ' && bytes[cursor] == b'\t') || (first == b'\t' && bytes[cursor] == b' '))
    {
        Some((cursor - 1, cursor + 1))
    } else {
        None
    }
}

fn smart_tabs_mixed_indent_range(bytes: &[u8], line: &LineInfo) -> Option<(usize, usize)> {
    let mut cursor = line.start;
    while cursor < line.content_end && bytes[cursor] == b'\t' {
        cursor += 1;
    }

    let spaces_start = cursor;
    while cursor < line.content_end && bytes[cursor] == b' ' {
        cursor += 1;
    }

    if cursor > spaces_start && cursor < line.content_end && bytes[cursor] == b'\t' {
        Some((cursor - 1, cursor + 1))
    } else {
        None
    }
}

fn is_ignored_mixed_indent_position(
    tokens: &[Token],
    lines: &[LineInfo],
    line_index: usize,
    start: usize,
) -> bool {
    tokens.iter().any(|token| {
        if start < token.start || start >= token.end {
            return false;
        }

        match token.kind {
            TokenKind::BlockComment => line_index > line_index_for_offset(lines, token.start),
            TokenKind::String
            | TokenKind::NoSubTemplate
            | TokenKind::TemplateHead
            | TokenKind::TemplateMiddle
            | TokenKind::TemplateTail => true,
            _ => false,
        }
    })
}

fn line_index_for_offset(lines: &[LineInfo], offset: usize) -> usize {
    lines
        .partition_point(|line| line.start <= offset)
        .saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{check_no_mixed_spaces_and_tabs, mixed_indent_range};
    use crate::native_stylistic::line_index::collect_lines;

    fn run(source: &str, options: serde_json::Value) -> Vec<crate::LintDiagnostic> {
        let lines = collect_lines(source);
        let mut diagnostics = Vec::new();
        check_no_mixed_spaces_and_tabs(source, &lines, &options, &mut diagnostics);
        diagnostics
    }

    fn no_options() -> Value {
        Value::Array(Vec::new())
    }

    fn smart_tabs_option() -> Value {
        Value::Array(Vec::from([Value::String("smart-tabs".to_owned())]))
    }

    fn boolean_smart_tabs_option() -> Value {
        Value::Array(Vec::from([Value::Bool(true)]))
    }

    fn reported_texts<'a>(
        source: &'a str,
        diagnostics: &'a [crate::LintDiagnostic],
    ) -> Vec<&'a str> {
        diagnostics
            .iter()
            .map(|diagnostic| {
                &source[diagnostic.range.start as usize..diagnostic.range.end as usize]
            })
            .collect()
    }

    #[test]
    fn reports_mixed_indentation() {
        let source = "function add() {\n\t return 1;\n   \treturn 2;\n}\n";
        let diagnostics = run(source, no_options());

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["mixedSpacesAndTabs", "mixedSpacesAndTabs"],
        );
        assert_eq!(reported_texts(source, &diagnostics), ["\t ", " \t"]);
    }

    #[test]
    fn honors_smart_tabs_alignment() {
        let source = "\t   aligned\n\t \tbad\n  \tbad\n";
        let diagnostics = run(source, smart_tabs_option());

        assert_eq!(reported_texts(source, &diagnostics), [" \t", " \t"]);
    }

    #[test]
    fn honors_deprecated_boolean_smart_tabs_option() {
        let source = "\t    aligned\n\t\t\t   \tbad\n";
        let diagnostics = run(source, boolean_smart_tabs_option());

        assert_eq!(reported_texts(source, &diagnostics), [" \t"]);
    }

    #[test]
    fn ignores_block_comment_continuations_and_literal_lines() {
        let source = "/*\n \t ignored\n*/\n'\\\n \t literal';\n`\n \t template\n`;\n \tcode;\n";
        let diagnostics = run(source, no_options());

        assert_eq!(reported_texts(source, &diagnostics), [" \t"]);
    }

    #[test]
    fn reports_first_comment_lines_but_ignores_continuations() {
        let source = "\t // comment\n \t/* first line */\n/*\n \t continuation\n*/\n";
        let diagnostics = run(source, no_options());

        assert_eq!(reported_texts(source, &diagnostics), ["\t ", " \t"]);
    }

    #[test]
    fn reports_template_substitution_indentation_but_ignores_template_body() {
        let source = "`foo${\n \t  5 }bar`;\nconst foo = `${console}\n\t body`;\n";
        let diagnostics = run(source, smart_tabs_option());

        assert_eq!(reported_texts(source, &diagnostics), [" \t"]);
    }

    #[test]
    fn uses_upstream_style_violation_ranges() {
        let source = "\t return x;\n   \tfoo\n\t\t\t   \tbar\n";
        let diagnostics = run(source, no_options());
        assert_eq!(reported_texts(source, &diagnostics), ["\t ", " \t", "\t "]);

        let smart_diagnostics = run(source, smart_tabs_option());
        assert_eq!(reported_texts(source, &smart_diagnostics), [" \t", " \t"]);
    }

    #[test]
    fn computes_default_indent_ranges() {
        let source = "\t foo\n   \tfoo\nfoo \t\n";
        let lines = collect_lines(source);

        let first = mixed_indent_range(source.as_bytes(), &lines[0], false).unwrap();
        let second = mixed_indent_range(source.as_bytes(), &lines[1], false).unwrap();

        assert_eq!(&source[first.0..first.1], "\t ");
        assert_eq!(&source[second.0..second.1], " \t");
        assert_eq!(
            mixed_indent_range(source.as_bytes(), &lines[2], false),
            None
        );
    }
}
