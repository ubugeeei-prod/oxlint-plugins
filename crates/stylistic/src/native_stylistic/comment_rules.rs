//! Comment-shape rules backed by the shared source token scan.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::{Scan, punct_is};
use super::lexer::{Token, TokenKind};

const RULE: &str = "multiline-comment-style";
const FIX_ID: &str = "fixStyle";
const FIX_MESSAGE: &str = "Apply the expected multiline comment style.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommentStyle {
    StarredBlock,
    BareBlock,
    SeparateLines,
}

#[derive(Clone, Copy, Debug)]
struct Config {
    style: CommentStyle,
    check_jsdoc: bool,
    check_exclamation: bool,
}

impl Config {
    fn from_options(options: &Value) -> Self {
        let values = match options {
            Value::Array(values) => values.as_slice(),
            Value::Null => &[],
            value => std::slice::from_ref(value),
        };
        let style = match values.first().and_then(Value::as_str) {
            Some("bare-block") => CommentStyle::BareBlock,
            Some("separate-lines") => CommentStyle::SeparateLines,
            _ => CommentStyle::StarredBlock,
        };
        let object = values.get(1).and_then(Value::as_object);
        Self {
            style,
            check_jsdoc: object
                .and_then(|value| value.get("checkJSDoc"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            check_exclamation: object
                .and_then(|value| value.get("checkExclamation"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SourceLine {
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct LineMap {
    lines: Vec<SourceLine>,
}

impl LineMap {
    fn new(source: &str) -> Self {
        let mut lines = Vec::new();
        let mut start = 0;
        let mut cursor = 0;
        let bytes = source.as_bytes();

        while cursor < bytes.len() {
            let break_len = linebreak_len(bytes, cursor);
            if break_len == 0 {
                cursor += source[cursor..].chars().next().map_or(1, char::len_utf8);
                continue;
            }
            lines.push(SourceLine { start, end: cursor });
            cursor += break_len;
            start = cursor;
        }
        lines.push(SourceLine {
            start,
            end: source.len(),
        });
        Self { lines }
    }

    fn index_at(&self, offset: usize) -> usize {
        self.lines
            .partition_point(|line| line.start <= offset)
            .saturating_sub(1)
    }

    fn text<'a>(&self, source: &'a str, index: usize) -> &'a str {
        let line = self.lines[index];
        &source[line.start..line.end]
    }
}

/// Enforces the stable `@stylistic` v5.10.0 multiline comment shapes.
pub(crate) fn check_multiline_comment_style(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let diagnostics_start = diagnostics.len();
    let config = Config::from_options(options);
    let lines = LineMap::new(scan.source());
    let ignored_jsx_comments = comments_in_jsx_text(scan);
    let eligible = eligible_comments(scan, &lines, &ignored_jsx_comments);
    let groups = comment_groups(scan, &lines, &eligible);

    for group in groups {
        let first = group[0];
        if group.len() == 1 && !is_multiline(first, &lines) {
            continue;
        }
        match config.style {
            CommentStyle::StarredBlock => check_starred_block(scan, &lines, &group, diagnostics),
            CommentStyle::BareBlock => check_bare_block(scan, &lines, &group, diagnostics),
            CommentStyle::SeparateLines => {
                check_separate_lines(scan, &lines, &group, config, diagnostics);
            }
        }
    }
    diagnostics[diagnostics_start..].sort_by_key(|diagnostic| {
        (
            diagnostic.range.start,
            diagnostic.range.end,
            diagnostic.message_id.clone(),
        )
    });
}

fn eligible_comments(scan: &Scan, lines: &LineMap, ignored_jsx_comments: &[bool]) -> Vec<usize> {
    let mut result = Vec::new();
    for (index, token) in scan.tokens().iter().enumerate() {
        if !token.kind.is_comment()
            || ignored_jsx_comments.get(index).copied().unwrap_or(false)
            || is_ignored_comment(comment_body(scan.source(), token))
        {
            continue;
        }

        let starts_own_line = index == 0
            || lines.index_at(scan.tokens()[index - 1].end.saturating_sub(1))
                < lines.index_at(token.start);
        if starts_own_line {
            result.push(index);
        }
    }
    result
}

fn comment_groups<'a>(scan: &'a Scan, lines: &LineMap, eligible: &[usize]) -> Vec<Vec<&'a Token>> {
    let mut groups: Vec<Vec<&Token>> = Vec::new();
    for &index in eligible {
        let token = &scan.tokens()[index];
        let joins_previous = token.kind == TokenKind::LineComment
            && groups.last().is_some_and(|group| {
                let previous = group[group.len() - 1];
                previous.kind == TokenKind::LineComment
                    && index > 0
                    && std::ptr::eq(previous, &scan.tokens()[index - 1])
                    && lines.index_at(previous.end.saturating_sub(1)) + 1
                        == lines.index_at(token.start)
            });
        if joins_previous {
            groups
                .last_mut()
                .expect("a previous group exists when joining")
                .push(token);
        } else {
            groups.push(std::iter::once(token).collect());
        }
    }
    groups
}

fn check_starred_block(
    scan: &Scan,
    line_map: &LineMap,
    group: &[&Token],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let first = group[0];
    let comment_lines = get_comment_lines(scan.source(), line_map, group);
    if comment_lines.iter().any(|line| line.contains("*/")) {
        return;
    }

    if group.len() > 1 {
        let replacement = if comment_lines.iter().any(|line| line.starts_with('/')) {
            None
        } else {
            Some(convert_to_starred_block(
                scan.source(),
                line_map,
                first,
                &comment_lines,
            ))
        };
        push(
            diagnostics,
            "expectedBlock",
            "Expected a block comment instead of consecutive line comments.",
            first.start,
            group[group.len() - 1].end,
            replacement.map(|replacement| (first.start, group[group.len() - 1].end, replacement)),
        );
        return;
    }

    let raw_lines = split_lines(comment_body(scan.source(), first));
    let initial_offset = initial_offset(scan.source(), line_map, first);
    let mut expected_prefix = initial_offset.to_owned();
    expected_prefix.push_str(" *");
    let first_value = raw_lines.first().copied().unwrap_or_default();

    if !is_optional_marker_whitespace(first_value) {
        let marker_len = first_value
            .chars()
            .next()
            .filter(|character| matches!(character, '*' | '!'))
            .map_or(0, char::len_utf8);
        let insert_at = first.start + 2 + marker_len;
        push(
            diagnostics,
            "startNewline",
            "Expected a linebreak after '/*'.",
            first.start,
            (first.start + 2).min(first.end),
            Some((insert_at, insert_at, concatenate(&["\n", &expected_prefix]))),
        );
    }

    if !raw_lines.last().copied().is_some_and(is_whitespace_only) {
        let closing_start = first.end.saturating_sub(2);
        push(
            diagnostics,
            "endNewline",
            "Expected a linebreak before '*/'.",
            closing_start,
            first.end,
            Some((
                closing_start,
                first.end,
                concatenate(&["\n", &expected_prefix, "/"]),
            )),
        );
    }

    let first_line = line_map.index_at(first.start);
    let last_line = line_map.index_at(first.end.saturating_sub(1));
    for line_index in first_line + 1..=last_line {
        let line_text = line_map.text(scan.source(), line_index);
        if line_text.starts_with(&expected_prefix) {
            continue;
        }

        let is_starred = line_text
            .trim_start_matches(char::is_whitespace)
            .starts_with('*');
        let (message_id, message) = if is_starred {
            (
                "alignment",
                "Expected this line to be aligned with the start of the comment.",
            )
        } else {
            ("missingStar", "Expected a '*' at the start of this line.")
        };
        let line = line_map.lines[line_index];
        let replacement_end;
        let replacement;
        if is_starred {
            replacement_end =
                line.start + whitespace_and_star_prefix_len(line_text).unwrap_or_default();
            replacement = expected_prefix.clone();
        } else {
            let whitespace_len = leading_whitespace_len(line_text);
            replacement_end = line.start + whitespace_len;
            let offset =
                missing_star_offset(scan.source(), line_map, first_line, &raw_lines, line_text);
            replacement = concatenate(&[&expected_prefix, &offset]);
        }
        push(
            diagnostics,
            message_id,
            message,
            line.start,
            line.end,
            Some((line.start, replacement_end, replacement)),
        );
    }
}

fn check_separate_lines(
    scan: &Scan,
    line_map: &LineMap,
    group: &[&Token],
    config: Config,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let first = group[0];
    let is_jsdoc = is_jsdoc(scan.source(), group);
    let is_exclamation = is_exclamation(scan.source(), group);
    if first.kind != TokenKind::BlockComment
        || (!config.check_jsdoc && is_jsdoc)
        || (!config.check_exclamation && is_exclamation)
    {
        return;
    }

    let Some(index) = scan
        .tokens()
        .iter()
        .position(|token| std::ptr::eq(token, first))
    else {
        return;
    };
    if scan.tokens().get(index + 1).is_some_and(|next| {
        line_map.index_at(first.end.saturating_sub(1)) == line_map.index_at(next.start)
    }) {
        return;
    }

    let mut comment_lines = get_comment_lines(scan.source(), line_map, group);
    if is_jsdoc || is_exclamation {
        comment_lines = comment_lines[1..comment_lines.len().saturating_sub(1)].to_vec();
    }
    let replacement = convert_to_separate_lines(scan.source(), line_map, first, &comment_lines);
    push(
        diagnostics,
        "expectedLines",
        "Expected multiple line comments instead of a block comment.",
        first.start,
        (first.start + 2).min(first.end),
        Some((first.start, first.end, replacement)),
    );
}

fn check_bare_block(
    scan: &Scan,
    line_map: &LineMap,
    group: &[&Token],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if is_jsdoc(scan.source(), group) || is_exclamation(scan.source(), group) {
        return;
    }
    let first = group[0];
    let comment_lines = get_comment_lines(scan.source(), line_map, group);

    if first.kind == TokenKind::LineComment
        && comment_lines.len() > 1
        && !comment_lines.iter().any(|line| line.contains("*/"))
    {
        let replacement = convert_to_bare_block(scan.source(), line_map, first, &comment_lines);
        push(
            diagnostics,
            "expectedBlock",
            "Expected a block comment instead of consecutive line comments.",
            first.start,
            group[group.len() - 1].end,
            Some((first.start, group[group.len() - 1].end, replacement)),
        );
    }

    if is_starred_block(scan.source(), group) {
        let replacement = convert_to_bare_block(scan.source(), line_map, first, &comment_lines);
        push(
            diagnostics,
            "expectedBareBlock",
            "Expected a block comment without padding stars.",
            first.start,
            (first.start + 2).min(first.end),
            Some((first.start, first.end, replacement)),
        );
    }
}

fn get_comment_lines(source: &str, line_map: &LineMap, group: &[&Token]) -> Vec<String> {
    if group[0].kind == TokenKind::LineComment {
        return process_separate_line_comments(source, group);
    }
    if is_starred_block(source, group) {
        return process_starred_block_comment(source, group[0]);
    }
    process_bare_block_comment(source, line_map, group[0])
}

fn process_separate_line_comments(source: &str, group: &[&Token]) -> Vec<String> {
    let values: Vec<&str> = group
        .iter()
        .map(|token| comment_body(source, token))
        .collect();
    let all_have_leading_space = values
        .iter()
        .all(|value| value.trim().is_empty() || value.starts_with(' '));
    values
        .into_iter()
        .map(|value| {
            if all_have_leading_space {
                value.strip_prefix(' ').unwrap_or(value).to_owned()
            } else {
                value.to_owned()
            }
        })
        .collect()
}

fn process_starred_block_comment(source: &str, comment: &Token) -> Vec<String> {
    let lines = split_lines(comment_body(source, comment));
    let inner = lines
        .get(1..lines.len().saturating_sub(1))
        .unwrap_or_default();
    let normalized: Vec<String> = inner
        .iter()
        .map(|line| {
            if is_whitespace_only(line) {
                String::new()
            } else {
                (*line).to_owned()
            }
        })
        .collect();
    let all_have_leading_space = normalized.iter().all(|line| {
        let without_prefix = remove_star_prefix(line, false);
        without_prefix.trim().is_empty() || without_prefix.starts_with(' ')
    });
    normalized
        .iter()
        .map(|line| remove_star_prefix(line, all_have_leading_space).to_owned())
        .collect()
}

fn process_bare_block_comment(source: &str, line_map: &LineMap, comment: &Token) -> Vec<String> {
    let lines: Vec<String> = split_lines(comment_body(source, comment))
        .into_iter()
        .map(|line| {
            if is_whitespace_only(line) {
                String::new()
            } else {
                line.to_owned()
            }
        })
        .collect();
    let leading_whitespace = concatenate(&[initial_offset(source, line_map, comment), "   "]);
    let mut offset = String::new();

    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() || index == 0 {
            continue;
        }
        let prefix_len = bare_line_prefix_len(line);
        if prefix_len < leading_whitespace.len() {
            let difference = leading_whitespace.len() - prefix_len;
            let new_offset =
                &leading_whitespace[leading_whitespace.len().saturating_sub(difference)..];
            if new_offset.len() > offset.len() {
                offset = new_offset.to_owned();
            }
        }
    }

    lines
        .iter()
        .map(|line| {
            let prefix_len = bare_line_prefix_len(line);
            let contents = &line[prefix_len..];
            if prefix_len > leading_whitespace.len() {
                let start = leading_whitespace.len().saturating_sub(offset.len());
                concatenate(&[&line[start.min(prefix_len)..prefix_len], contents])
            } else if prefix_len < leading_whitespace.len() {
                let start = leading_whitespace.len().min(prefix_len);
                concatenate(&[&line[start..prefix_len], contents])
            } else {
                contents.to_owned()
            }
        })
        .collect()
}

fn convert_to_starred_block(
    source: &str,
    line_map: &LineMap,
    first: &Token,
    lines: &[String],
) -> String {
    let offset = initial_offset(source, line_map, first);
    let mut replacement = String::from("/*\n");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            replacement.push('\n');
        }
        replacement.push_str(offset);
        replacement.push_str(" * ");
        replacement.push_str(line);
    }
    replacement.push('\n');
    replacement.push_str(offset);
    replacement.push_str(" */");
    replacement
}

fn convert_to_separate_lines(
    source: &str,
    line_map: &LineMap,
    first: &Token,
    lines: &[String],
) -> String {
    let offset = initial_offset(source, line_map, first);
    let mut replacement = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            replacement.push('\n');
            replacement.push_str(offset);
        }
        replacement.push_str("// ");
        replacement.push_str(line);
    }
    replacement
}

fn convert_to_bare_block(
    source: &str,
    line_map: &LineMap,
    first: &Token,
    lines: &[String],
) -> String {
    let offset = initial_offset(source, line_map, first);
    let mut replacement = String::from("/* ");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            replacement.push('\n');
            replacement.push_str(offset);
            replacement.push_str("   ");
        }
        replacement.push_str(line);
    }
    replacement.push_str(" */");
    replacement
}

fn concatenate(parts: &[&str]) -> String {
    let mut result = String::with_capacity(parts.iter().map(|part| part.len()).sum());
    for part in parts {
        result.push_str(part);
    }
    result
}

fn initial_offset<'a>(source: &'a str, line_map: &LineMap, comment: &Token) -> &'a str {
    let line = line_map.lines[line_map.index_at(comment.start)];
    &source[line.start..comment.start]
}

fn is_starred_block(source: &str, group: &[&Token]) -> bool {
    let first = group[0];
    if first.kind != TokenKind::BlockComment {
        return false;
    }
    let lines = split_lines(comment_body(source, first));
    !lines.is_empty()
        && lines.iter().enumerate().all(|(index, line)| {
            if index == 0 || index + 1 == lines.len() {
                is_whitespace_only(line)
            } else {
                line.trim_start_matches(char::is_whitespace)
                    .starts_with('*')
            }
        })
}

fn is_jsdoc(source: &str, group: &[&Token]) -> bool {
    marker_block(source, group, '*')
}

fn is_exclamation(source: &str, group: &[&Token]) -> bool {
    marker_block(source, group, '!')
}

fn marker_block(source: &str, group: &[&Token], marker: char) -> bool {
    let first = group[0];
    if first.kind != TokenKind::BlockComment {
        return false;
    }
    let lines = split_lines(comment_body(source, first));
    let Some(first_line) = lines.first() else {
        return false;
    };
    let marker_matches = first_line
        .strip_prefix(marker)
        .is_some_and(is_whitespace_only);
    marker_matches
        && lines
            .get(1..lines.len().saturating_sub(1))
            .unwrap_or_default()
            .iter()
            .all(|line| {
                let trimmed = line.trim_start_matches(char::is_whitespace);
                line.len() > trimmed.len() && line[..line.len() - trimmed.len()].contains(' ')
            })
        && lines.last().copied().is_some_and(is_whitespace_only)
}

fn is_ignored_comment(value: &str) -> bool {
    let trimmed = value.trim_start_matches(char::is_whitespace);
    trimmed.starts_with("eslint")
        || trimmed.starts_with("jscs")
        || starts_with_word_space(trimmed, "jshint")
        || starts_with_word_space(trimmed, "jslint")
        || starts_with_word_space(trimmed, "istanbul")
        || starts_with_word_space(trimmed, "global")
        || starts_with_word_space(trimmed, "globals")
        || starts_with_word_space(trimmed, "exported")
}

fn starts_with_word_space(value: &str, word: &str) -> bool {
    value
        .strip_prefix(word)
        .and_then(|rest| rest.chars().next())
        .is_some_and(char::is_whitespace)
}

fn is_multiline(token: &Token, lines: &LineMap) -> bool {
    lines.index_at(token.start) != lines.index_at(token.end.saturating_sub(1))
}

fn comment_body<'a>(source: &'a str, token: &Token) -> &'a str {
    match token.kind {
        TokenKind::LineComment => &source[token.start + 2..token.end],
        TokenKind::BlockComment if token.end >= token.start + 4 => {
            &source[token.start + 2..token.end - 2]
        }
        TokenKind::BlockComment => &source[token.start + 2..token.end],
        _ => "",
    }
}

fn split_lines(value: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let bytes = value.as_bytes();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        let break_len = linebreak_len(bytes, cursor);
        if break_len == 0 {
            cursor += value[cursor..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        lines.push(&value[start..cursor]);
        cursor += break_len;
        start = cursor;
    }
    lines.push(&value[start..]);
    lines
}

fn linebreak_len(bytes: &[u8], cursor: usize) -> usize {
    match bytes.get(cursor..) {
        Some([b'\r', b'\n', ..]) => 2,
        Some([b'\r' | b'\n', ..]) => 1,
        Some([0xe2, 0x80, 0xa8 | 0xa9, ..]) => 3,
        _ => 0,
    }
}

fn is_whitespace_only(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

fn is_optional_marker_whitespace(value: &str) -> bool {
    let value = value
        .strip_prefix('*')
        .or_else(|| value.strip_prefix('!'))
        .unwrap_or(value);
    is_whitespace_only(value)
}

fn leading_whitespace_len(value: &str) -> usize {
    value
        .char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))
        .unwrap_or(value.len())
}

fn whitespace_and_star_prefix_len(value: &str) -> Option<usize> {
    let whitespace = leading_whitespace_len(value);
    value[whitespace..]
        .starts_with('*')
        .then_some(whitespace + 1)
}

fn bare_line_prefix_len(value: &str) -> usize {
    let mut cursor = leading_whitespace_len(value);
    if value[cursor..].starts_with('*') {
        cursor += 1;
        cursor += leading_whitespace_len(&value[cursor..]);
    }
    cursor
}

fn remove_star_prefix(value: &str, remove_one_space: bool) -> &str {
    let mut cursor = leading_whitespace_len(value);
    if value[cursor..].starts_with('*') {
        cursor += 1;
        if remove_one_space && value[cursor..].starts_with(' ') {
            cursor += 1;
        }
    }
    &value[cursor..]
}

fn missing_star_offset(
    source: &str,
    line_map: &LineMap,
    first_line: usize,
    raw_lines: &[&str],
    current_line: &str,
) -> String {
    let current_prefix = &current_line[..leading_whitespace_len(current_line)];
    for (index, line) in raw_lines.iter().enumerate() {
        if is_whitespace_only(line) {
            continue;
        }
        let align_line = line_map.text(source, first_line + index);
        let (prefix_len, trailing_whitespace) = alignment_prefix(align_line);
        let mut offset = String::new();
        if current_prefix.len() > prefix_len {
            offset.push_str(&current_prefix[prefix_len..]);
        }
        offset.push_str(trailing_whitespace);
        if current_line
            .trim_start_matches(char::is_whitespace)
            .starts_with('/')
            && offset.is_empty()
        {
            offset.push(' ');
        }
        return offset;
    }
    String::new()
}

fn alignment_prefix(value: &str) -> (usize, &str) {
    let whitespace = leading_whitespace_len(value);
    let mut prefix_end = whitespace;
    if value[prefix_end..].starts_with("/*") {
        prefix_end += 2;
    } else if value[prefix_end..].starts_with('*') {
        prefix_end += 1;
    }
    let trailing_len = leading_whitespace_len(&value[prefix_end..]);
    (
        prefix_end + trailing_len,
        &value[prefix_end..prefix_end + trailing_len],
    )
}

fn push(
    diagnostics: &mut Vec<LintDiagnostic>,
    message_id: &str,
    message: &str,
    start: usize,
    end: usize,
    fix: Option<(usize, usize, String)>,
) {
    let Ok(start) = u32::try_from(start) else {
        return;
    };
    let Ok(end) = u32::try_from(end) else {
        return;
    };
    let suggestions = fix
        .and_then(|(fix_start, fix_end, replacement)| {
            Some(LintSuggestion {
                message_id: FIX_ID.to_owned(),
                message: FIX_MESSAGE.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(
                    TextRange::new(u32::try_from(fix_start).ok()?, u32::try_from(fix_end).ok()?),
                    replacement,
                ))
                .collect(),
            })
        })
        .into_iter()
        .collect();
    diagnostics.push(LintDiagnostic {
        rule_name: RULE.to_owned(),
        message_id: message_id.to_owned(),
        message: message.to_owned(),
        range: TextRange::new(start, end),
        suggestions,
        data: BTreeMap::new(),
    });
}

/// Marks block/line-comment tokens that are actually raw JSX child text.
///
/// The shared lexer cannot emit a dedicated JSX-text token, so this small state
/// machine recognizes conservative JSX roots and skips braced expressions.
fn comments_in_jsx_text(scan: &Scan) -> Vec<bool> {
    let tokens = scan.tokens();
    let mut ignored: Vec<bool> = std::iter::repeat_n(false, tokens.len()).collect();
    let mut jsx_depth = 0_usize;
    let mut index = 0_usize;

    while index < tokens.len() {
        if jsx_depth > 0 && punct_is(&tokens[index], scan.source(), "{") {
            if let Some(partner) = scan.partner(index) {
                index = partner + 1;
                continue;
            }
        }
        if !punct_is(&tokens[index], scan.source(), "<") {
            if jsx_depth > 0 && tokens[index].kind.is_comment() {
                ignored[index] = true;
            }
            index += 1;
            continue;
        }

        let Some(close) = find_jsx_tag_close(scan, index + 1) else {
            index += 1;
            continue;
        };
        let next = scan.next_significant(index);
        let closing = next.is_some_and(|next| punct_is(&tokens[next], scan.source(), "/"));
        let fragment = next.is_some_and(|next| punct_is(&tokens[next], scan.source(), ">"));
        let opening = next.is_some_and(|next| tokens[next].kind == TokenKind::Identifier);
        if closing {
            jsx_depth = jsx_depth.saturating_sub(1);
        } else if (fragment || opening)
            && (jsx_depth > 0 || can_start_jsx_root(scan, index))
            && has_matching_jsx_close(scan, index, close, fragment)
        {
            let self_closing = scan
                .prev_significant(close)
                .is_some_and(|previous| punct_is(&tokens[previous], scan.source(), "/"));
            if !self_closing {
                jsx_depth += 1;
            }
        }
        index = close + 1;
    }
    ignored
}

fn can_start_jsx_root(scan: &Scan, open: usize) -> bool {
    let Some(previous) = scan.prev_significant(open) else {
        return true;
    };
    let token = &scan.tokens()[previous];
    if token.kind == TokenKind::Identifier {
        return matches!(
            scan.token_text(previous),
            "return" | "yield" | "case" | "default" | "await" | "typeof" | "void" | "in" | "of"
        );
    }
    token.kind == TokenKind::Punctuator
        && matches!(
            scan.token_text(previous),
            "=" | "=>" | "(" | "[" | "{" | "," | ":" | ";" | "?" | ">" | "&&" | "||" | "??" | "!"
        )
}

fn has_matching_jsx_close(scan: &Scan, open: usize, open_close: usize, fragment: bool) -> bool {
    let tokens = scan.tokens();
    let name = scan
        .next_significant(open)
        .filter(|_| !fragment)
        .map(|index| scan.token_text(index));
    let mut index = open_close + 1;
    while index < tokens.len() {
        if punct_is(&tokens[index], scan.source(), "<") {
            let Some(slash) = scan.next_significant(index) else {
                return false;
            };
            if punct_is(&tokens[slash], scan.source(), "/") {
                let Some(after_slash) = scan.next_significant(slash) else {
                    return false;
                };
                if fragment {
                    if punct_is(&tokens[after_slash], scan.source(), ">") {
                        return true;
                    }
                } else if name.is_some_and(|name| scan.token_text(after_slash) == name) {
                    return true;
                }
            }
        }
        index += 1;
    }
    false
}

fn find_jsx_tag_close(scan: &Scan, start: usize) -> Option<usize> {
    let tokens = scan.tokens();
    let mut index = start;
    while index < tokens.len() {
        if punct_is(&tokens[index], scan.source(), "{") {
            index = scan.partner(index)?.saturating_add(1);
            continue;
        }
        if punct_is(&tokens[index], scan.source(), ">") {
            return Some(index);
        }
        if punct_is(&tokens[index], scan.source(), "<") {
            return None;
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(style: &str) -> Value {
        Value::Array(std::iter::once(Value::String(style.to_owned())).collect())
    }

    fn separate_options(check_jsdoc: bool, check_exclamation: bool) -> Value {
        let source = match (check_jsdoc, check_exclamation) {
            (true, true) => r#"["separate-lines",{"checkJSDoc":true,"checkExclamation":true}]"#,
            (true, false) => r#"["separate-lines",{"checkJSDoc":true,"checkExclamation":false}]"#,
            (false, true) => r#"["separate-lines",{"checkJSDoc":false,"checkExclamation":true}]"#,
            (false, false) => r#"["separate-lines",{"checkJSDoc":false,"checkExclamation":false}]"#,
        };
        serde_json::from_str(source).expect("static multiline comment options are valid JSON")
    }

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check_multiline_comment_style(&scan, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(diagnostics: &[LintDiagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id.as_str())
            .collect()
    }

    fn fixes(diagnostics: &[LintDiagnostic]) -> Vec<(TextRange, &str)> {
        diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .map(|suggestion| {
                let fix = &suggestion.fixes[0];
                (fix.range, fix.replacement_text.as_str())
            })
            .collect()
    }

    fn fixed_output(source: &str, options: Value) -> String {
        let mut output = source.to_owned();
        for _ in 0..32 {
            let diagnostics = run(&output, options.clone());
            let Some(fix) = diagnostics
                .iter()
                .find_map(|diagnostic| diagnostic.suggestions.first())
                .and_then(|suggestion| suggestion.fixes.first())
            else {
                return output;
            };
            output.replace_range(
                usize::try_from(fix.range.start).unwrap()..usize::try_from(fix.range.end).unwrap(),
                &fix.replacement_text,
            );
        }
        panic!("multiline-comment-style fixes did not converge for {source:?}");
    }

    #[test]
    fn accepts_upstream_default_starred_block_fixtures() {
        for source in [
            "/*\n * this is\n * a comment\n */",
            "/**\n * this is\n * a JSDoc comment\n */",
            "/*!\n * this is\n * an exclamation comment\n */",
            "/*! this is a single line exclamation comment */",
            "/* eslint semi: [\n  \"error\"\n] */",
            "// this is a single-line comment",
            "/* foo */",
            "\t\t/**\n\t\t * this comment\n\t\t * is tab-aligned\n\t\t */",
            "/**\r\n * this comment\r\n * uses windows linebreaks\r\n */",
            "/**\u{2029} * this comment\u{2029} * uses paragraph separators\u{2029} */",
            "foo(/* this is an\n    inline comment */);",
            "// The following line comment\n// contains '*/'.",
            "let x = 5; // first number\n// second number\nlet y = 10;",
        ] {
            assert!(
                run(source, Value::Null).is_empty(),
                "default rejected {source:?}"
            );
        }
    }

    #[test]
    fn accepts_upstream_style_specific_valid_fixtures() {
        let valid = [
            ("starred-block", "/*\n * foo\n */\n// single\n/* single */"),
            (
                "bare-block",
                "/* This is\n   a comment */\n/**\n * JSDoc\n */\n/*!\n * license\n */",
            ),
            (
                "separate-lines",
                "// this is\n// a comment\n/**\n * JSDoc\n */\n/*!\n * license\n */",
            ),
            (
                "separate-lines",
                "/* this is\n   a comment */ foo;\n// first\n\n// second",
            ),
        ];
        for (style, source) in valid {
            assert!(
                run(source, options(style)).is_empty(),
                "{style} rejected {source:?}"
            );
        }
    }

    #[test]
    fn converts_consecutive_line_comments_to_starred_blocks() {
        let source = "  // these are\n  // line comments";
        let diagnostics = run(source, Value::Null);
        assert_eq!(ids(&diagnostics), ["expectedBlock"]);
        assert_eq!(
            fixes(&diagnostics),
            [(
                TextRange::new(2, u32::try_from(source.len()).unwrap()),
                "/*\n   * these are\n   * line comments\n   */"
            )]
        );

        let diagnostics = run("//  foo\n//\n//    baz\n// qux", Value::Null);
        assert_eq!(ids(&diagnostics), ["expectedBlock"]);
        assert_eq!(
            fixes(&diagnostics)[0].1,
            "/*\n *  foo\n * \n *    baz\n * qux\n */"
        );
    }

    #[test]
    fn preserves_no_fix_cases_containing_comment_terminators_or_slashes() {
        assert!(run("// foo\n// contains */", Value::Null).is_empty());
        for source in ["//foo\n///bar", "////foo\n//`bar`"] {
            let diagnostics = run(source, Value::Null);
            assert_eq!(ids(&diagnostics), ["expectedBlock"], "{source:?}");
            assert!(diagnostics[0].suggestions.is_empty(), "{source:?}");
        }
    }

    #[test]
    fn reports_and_fixes_starred_block_boundaries() {
        let start = "/* this block\n * continues\n */";
        let diagnostics = run(start, Value::Null);
        assert_eq!(ids(&diagnostics), ["startNewline"]);
        assert_eq!(fixes(&diagnostics), [(TextRange::new(2, 2), "\n *")]);

        let end = "/*\n * this block\n * has no final linebreak*/";
        let diagnostics = run(end, Value::Null);
        assert_eq!(ids(&diagnostics), ["endNewline"]);
        assert_eq!(fixes(&diagnostics), [(TextRange::new(42, 44), "\n */")]);

        let jsdoc = "/** JSDoc\n * continues\n */";
        assert_eq!(
            fixes(&run(jsdoc, Value::Null))[0],
            (TextRange::new(3, 3), "\n *")
        );
        let exclamation = "/*! license\n * continues\n */";
        assert_eq!(
            fixes(&run(exclamation, Value::Null))[0],
            (TextRange::new(3, 3), "\n *")
        );
    }

    #[test]
    fn reports_missing_and_misaligned_stars_with_exact_fixes() {
        let source = "  /*\n   * good\n     missing\n       * misaligned\n    */";
        let diagnostics = run(source, Value::Null);
        assert_eq!(ids(&diagnostics), ["missingStar", "alignment", "alignment"]);
        assert_eq!(
            fixes(&diagnostics),
            [
                (TextRange::new(15, 20), "   * "),
                (TextRange::new(28, 36), "   *"),
                (TextRange::new(48, 53), "   *")
            ]
        );
    }

    #[test]
    fn converts_starred_and_bare_blocks_to_separate_lines() {
        let starred = "  /*\n   * foo\n   *\n   * bar\n   */";
        let diagnostics = run(starred, options("separate-lines"));
        assert_eq!(ids(&diagnostics), ["expectedLines"]);
        assert_eq!(
            fixes(&diagnostics),
            [(TextRange::new(2, 33), "// foo\n  // \n  // bar")]
        );

        let bare = "  /* foo\n     bar */";
        let diagnostics = run(bare, options("separate-lines"));
        assert_eq!(ids(&diagnostics), ["expectedLines"]);
        assert_eq!(
            fixes(&diagnostics),
            [(TextRange::new(2, 20), "// foo\n  // bar ")]
        );
    }

    #[test]
    fn ports_upstream_bare_block_indentation_fixtures() {
        let deeply_indented = "                /* This is\n                         a comment */";
        assert_eq!(
            fixed_output(deeply_indented, Value::Null),
            "                /*\n                 * This is\n                 *       a comment \n                 */"
        );
        assert_eq!(
            fixed_output(deeply_indented, options("separate-lines")),
            "                // This is\n                //       a comment "
        );

        let json = concat!(
            "                /* {\n",
            "                       \"foo\": 1,\n",
            "                       \"bar\": 2\n",
            "                   } */"
        );
        assert_eq!(
            fixed_output(json, Value::Null),
            concat!(
                "                /*\n",
                "                 * {\n",
                "                 *     \"foo\": 1,\n",
                "                 *     \"bar\": 2\n",
                "                 * } \n",
                "                 */"
            )
        );
        assert_eq!(
            fixed_output(json, options("separate-lines")),
            concat!(
                "                // {\n",
                "                //     \"foo\": 1,\n",
                "                //     \"bar\": 2\n",
                "                // } "
            )
        );

        let uneven = "  /* foo\n          bar\n x */";
        assert_eq!(
            fixed_output(uneven, Value::Null),
            "  /*\n   * foo\n   *      bar\n   * x \n   */"
        );
        assert_eq!(
            fixed_output(uneven, options("separate-lines")),
            "  // foo\n  //          bar\n  // x "
        );
    }

    #[test]
    fn separate_lines_honors_jsdoc_and_exclamation_options() {
        let jsdoc = "/**\n * JSDoc\n * Comment\n */";
        assert!(run(jsdoc, options("separate-lines")).is_empty());
        let diagnostics = run(jsdoc, separate_options(true, false));
        assert_eq!(ids(&diagnostics), ["expectedLines"]);
        assert_eq!(fixes(&diagnostics)[0].1, "// JSDoc\n// Comment");

        let exclamation = "/*!\n * Exclamation\n * Comment\n */";
        assert!(run(exclamation, options("separate-lines")).is_empty());
        let diagnostics = run(exclamation, separate_options(false, true));
        assert_eq!(ids(&diagnostics), ["expectedLines"]);
        assert_eq!(fixes(&diagnostics)[0].1, "// Exclamation\n// Comment");
    }

    #[test]
    fn separate_lines_skips_blocks_followed_by_same_line_tokens() {
        for source in [
            "/* foo\n   bar */ value;",
            "call(/* foo\n        bar */);",
            "/* foo\n   bar *//* next\n              block */",
        ] {
            assert!(
                run(source, options("separate-lines")).is_empty(),
                "{source:?}"
            );
        }
    }

    #[test]
    fn converts_line_and_starred_comments_to_bare_blocks() {
        let lines = "  // foo\n// bar";
        let diagnostics = run(lines, options("bare-block"));
        assert_eq!(ids(&diagnostics), ["expectedBlock"]);
        assert_eq!(
            fixes(&diagnostics),
            [(TextRange::new(2, 15), "/* foo\n     bar */")]
        );

        let starred = "/*\n *    foo\n *  bar\n * qux\n */";
        let diagnostics = run(starred, options("bare-block"));
        assert_eq!(ids(&diagnostics), ["expectedBareBlock"]);
        assert_eq!(fixes(&diagnostics)[0].1, "/*    foo\n    bar\n   qux */");
    }

    #[test]
    fn bare_block_preserves_jsdoc_and_exclamation_comments() {
        for source in [
            "/**\n * JSDoc\n */",
            "/*!\n * license\n */",
            "/*! one-line license */",
        ] {
            assert!(run(source, options("bare-block")).is_empty(), "{source:?}");
        }
    }

    #[test]
    fn respects_comment_group_boundaries_and_default_ignores() {
        let source = concat!(
            "// first\n// second\n\n",
            "// eslint-disable\n// third\n// fourth\n",
            "value; // inline\n// fifth\n"
        );
        let diagnostics = run(source, Value::Null);
        assert_eq!(ids(&diagnostics), ["expectedBlock", "expectedBlock"]);
        assert_eq!(diagnostics[0].range, TextRange::new(0, 18));
        assert_eq!(diagnostics[1].range, TextRange::new(38, 56));
    }

    #[test]
    fn supports_crlf_cr_and_unicode_line_terminators() {
        for source in [
            "// first\r\n// second",
            "// first\r// second",
            "// first\u{2028}// second",
            "// first\u{2029}// second",
            "/*\u{2028} * first\u{2028} * second\u{2028} */",
            "/*\u{2029} * first\u{2029} * second\u{2029} */",
        ] {
            let expected = source.starts_with("//");
            assert_eq!(!run(source, Value::Null).is_empty(), expected, "{source:?}");
        }
    }

    #[test]
    fn preserves_utf8_ranges_for_utf16_bridge_mapping() {
        let source = "日本語\n  // première\n  // deuxième";
        let diagnostics = run(source, Value::Null);
        assert_eq!(ids(&diagnostics), ["expectedBlock"]);
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(
                u32::try_from(source.find("// première").unwrap()).unwrap(),
                u32::try_from(source.len()).unwrap()
            )
        );
        assert_eq!(
            fixes(&diagnostics)[0].1,
            "/*\n   * première\n   * deuxième\n   */"
        );
    }

    #[test]
    fn ignores_comment_like_text_in_literals_regexes_and_jsx_children() {
        for source in [
            "const a = '/* first\\nsecond */';",
            "const b = `// first\\n// second`;",
            "const c = /\\/\\* first/;",
            "const view = <div>/* first\nsecond */</div>;",
            "const view = <><span>// first\n// second</span></>;",
        ] {
            assert!(run(source, Value::Null).is_empty(), "{source:?}");
        }
    }

    #[test]
    fn checks_real_comments_inside_typescript_and_jsx_expressions() {
        let source = concat!(
            "type Box<T> = {\n// first\n// second\nvalue: T\n};\n",
            "const generic = <T>(value: T) => {\n// first\n// second\nreturn value;\n};\n",
            "const view = <div>{\n/* first\nsecond */\n}</div>;\n"
        );
        assert_eq!(
            ids(&run(source, Value::Null)),
            [
                "expectedBlock",
                "expectedBlock",
                "startNewline",
                "missingStar",
                "endNewline"
            ]
        );
    }

    #[test]
    fn replays_every_pinned_upstream_fixture_with_exact_diagnostics_and_output() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../npm/stylistic/test/fixtures/multiline-comment-style.json"
        )))
        .expect("committed multiline-comment-style fixture is valid JSON");
        let generated = fixture
            .get("__generated")
            .and_then(|value| value.get("inventory"))
            .expect("fixture has an inventory");
        assert_eq!(generated.get("valid").and_then(Value::as_u64), Some(55));
        assert_eq!(generated.get("invalid").and_then(Value::as_u64), Some(69));
        assert_eq!(
            generated.get("diagnostics").and_then(Value::as_u64),
            Some(107)
        );

        let valid = fixture
            .get("valid")
            .and_then(Value::as_array)
            .expect("fixture has valid cases");
        for (index, test_case) in valid.iter().enumerate() {
            let code = test_case
                .get("code")
                .and_then(Value::as_str)
                .expect("valid case has code");
            let diagnostics = run(code, fixture_options(test_case));
            assert!(
                diagnostics.is_empty(),
                "upstream valid case {index} produced {diagnostics:#?}"
            );
        }

        let invalid = fixture
            .get("invalid")
            .and_then(Value::as_array)
            .expect("fixture has invalid cases");
        let mut diagnostic_count = 0_usize;
        for (index, test_case) in invalid.iter().enumerate() {
            let code = test_case
                .get("code")
                .and_then(Value::as_str)
                .expect("invalid case has code");
            let expected = test_case
                .get("expectedDiagnostics")
                .and_then(Value::as_array)
                .expect("invalid case has exact diagnostics");
            let options = fixture_options(test_case);
            let diagnostics = run(code, options.clone());
            diagnostic_count += diagnostics.len();
            assert_eq!(
                diagnostics.len(),
                expected.len(),
                "upstream invalid case {index} diagnostic count"
            );

            for (diagnostic_index, (actual, expected)) in
                diagnostics.iter().zip(expected).enumerate()
            {
                let expected_range = expected
                    .get("range")
                    .and_then(Value::as_array)
                    .expect("expected diagnostic has a range");
                let expected_start = utf16_offset_to_byte(
                    code,
                    usize::try_from(expected_range[0].as_u64().unwrap()).unwrap(),
                );
                let expected_end = utf16_offset_to_byte(
                    code,
                    usize::try_from(expected_range[1].as_u64().unwrap()).unwrap(),
                );
                assert_eq!(
                    actual.message_id,
                    expected.get("messageId").and_then(Value::as_str).unwrap(),
                    "upstream invalid case {index}, diagnostic {diagnostic_index} message ID"
                );
                assert_eq!(
                    actual.message,
                    expected.get("message").and_then(Value::as_str).unwrap(),
                    "upstream invalid case {index}, diagnostic {diagnostic_index} message"
                );
                assert!(
                    actual.data.is_empty(),
                    "upstream invalid case {index}, diagnostic {diagnostic_index} data"
                );
                assert_eq!(
                    actual.range,
                    TextRange::new(
                        u32::try_from(expected_start).unwrap(),
                        u32::try_from(expected_end).unwrap()
                    ),
                    "upstream invalid case {index}, diagnostic {diagnostic_index} range"
                );
            }

            match test_case.get("output").expect("invalid case has output") {
                Value::Null => assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.suggestions.is_empty()),
                    "upstream invalid case {index} is explicitly unfixable"
                ),
                Value::String(expected_output) => {
                    let actual_output = apply_all_fixes(code, &diagnostics)
                        .expect("fixable upstream case exposes fixes");
                    assert_eq!(
                        actual_output, *expected_output,
                        "upstream invalid case {index} fixed output"
                    );
                    assert!(
                        run(&actual_output, options).is_empty(),
                        "upstream invalid case {index} output must converge"
                    );
                }
                _ => panic!("upstream invalid case {index} output must be a string or null"),
            }
        }
        assert_eq!(diagnostic_count, 107);
    }

    fn fixture_options(test_case: &Value) -> Value {
        test_case
            .get("options")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()))
    }

    fn apply_all_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .filter_map(|suggestion| suggestion.fixes.first())
            .map(|fix| {
                (
                    usize::try_from(fix.range.start).unwrap(),
                    usize::try_from(fix.range.end).unwrap(),
                    fix.replacement_text.clone(),
                )
            })
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|(start, end, _)| (*start, *end));

        let mut accepted = Vec::new();
        let mut last_end = 0_usize;
        for fix in fixes {
            if !accepted.is_empty() && fix.0 < last_end {
                continue;
            }
            last_end = fix.1;
            accepted.push(fix);
        }

        let mut output = source.to_owned();
        for (start, end, replacement) in accepted.into_iter().rev() {
            output.replace_range(start..end, &replacement);
        }
        Some(output)
    }

    fn utf16_offset_to_byte(source: &str, target: usize) -> usize {
        let mut utf16_offset = 0_usize;
        for (byte_offset, character) in source.char_indices() {
            if utf16_offset == target {
                return byte_offset;
            }
            utf16_offset += character.len_utf16();
        }
        assert_eq!(utf16_offset, target, "UTF-16 offset lands on a boundary");
        source.len()
    }
}
