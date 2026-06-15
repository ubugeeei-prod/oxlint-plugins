//! Parsing of ESLint directive comments.
//!
//! Clean-room implementation of the directive-comment grammar that ESLint and
//! `@eslint-community/eslint-plugin-eslint-comments` recognize. No upstream
//! source is copied; only the observable grammar (kinds, description divider,
//! line/block restrictions) is reproduced and covered with snapshot tests.

use oxlint_plugins_carton::CompactString;

/// Whether the host comment token is a line (`//`) or block (`/* */`) comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentKind {
    Line,
    Block,
}

/// The directive keyword that introduces a directive comment.
static DIRECTIVE_KINDS: phf::Set<&'static str> = phf::phf_set! {
    "eslint",
    "eslint-env",
    "eslint-enable",
    "eslint-disable",
    "eslint-disable-line",
    "eslint-disable-next-line",
    "exported",
    "global",
    "globals",
};

/// A parsed directive comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDirective {
    /// The directive keyword, e.g. `eslint-disable`.
    pub kind: CompactString,
    /// The trimmed text following the keyword (rule ids / globals / config).
    pub value: CompactString,
    /// The trailing ` -- description` text, if present.
    pub description: Option<CompactString>,
}

impl ParsedDirective {
    /// `true` when no rule names / value follow the keyword.
    pub fn has_empty_value(&self) -> bool {
        self.value.is_empty()
    }
}

/// Returns `true` when the keyword is only valid as a line comment directive.
fn is_line_comment_kind(kind: &str) -> bool {
    kind == "eslint-disable-line" || kind == "eslint-disable-next-line"
}

/// Parse the raw comment body text as a directive, ignoring the host comment
/// type. Mirrors upstream `parseDirectiveText`.
pub fn parse_directive_text(text_to_parse: &str) -> Option<ParsedDirective> {
    let (text, description) = divide_directive_comment(text_to_parse);
    let keyword = leading_keyword(text)?;

    if !DIRECTIVE_KINDS.contains(keyword) {
        return None;
    }

    let value = text[keyword.len()..].trim();

    Some(ParsedDirective {
        kind: CompactString::from(keyword),
        value: CompactString::from(value),
        description: description.map(CompactString::from),
    })
}

/// Parse a comment token as a directive, honoring line/block restrictions.
/// Mirrors upstream `parseDirectiveComment`.
///
/// `same_line` indicates whether the comment starts and ends on the same line;
/// an `eslint-disable-line` directive must not span multiple lines.
pub fn parse_directive_comment(
    comment_kind: CommentKind,
    value: &str,
    same_line: bool,
) -> Option<ParsedDirective> {
    let parsed = parse_directive_text(value)?;

    let line_supported = is_line_comment_kind(&parsed.kind);
    if comment_kind == CommentKind::Line && !line_supported {
        return None;
    }

    if parsed.kind == "eslint-disable-line" && !same_line {
        return None;
    }

    Some(parsed)
}

/// Split a directive comment body into directive text and an optional trailing
/// description. The divider is whitespace, two or more hyphens, then
/// whitespace (e.g. `eslint-disable foo -- because reasons`).
fn divide_directive_comment(value: &str) -> (&str, Option<&str>) {
    let Some(first) = find_separator(value, 0) else {
        return (value.trim(), None);
    };

    let text = value[..first.0].trim();
    let description_end = find_separator(value, first.1)
        .map(|(start, _)| start)
        .unwrap_or(value.len());
    let description = value[first.1..description_end].trim();

    (text, Some(description))
}

/// Find the byte range of the first description divider at or after `from`.
fn find_separator(value: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = value.as_bytes();
    let mut iter = value
        .char_indices()
        .skip_while(|(index, _)| *index < from)
        .peekable();

    while let Some(&(start, ch)) = iter.peek() {
        iter.next();
        if !ch.is_whitespace() {
            continue;
        }

        let mut cursor = start + ch.len_utf8();
        let mut hyphens = 0;
        while bytes.get(cursor) == Some(&b'-') {
            hyphens += 1;
            cursor += 1;
        }
        if hyphens < 2 {
            continue;
        }

        match value[cursor..].chars().next() {
            Some(next) if next.is_whitespace() => {
                return Some((start, cursor + next.len_utf8()));
            }
            _ => continue,
        }
    }

    None
}

/// The keyword is the run of characters up to the first whitespace.
fn leading_keyword(text: &str) -> Option<&str> {
    if text.is_empty() {
        return None;
    }
    let end = text
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map_or(text.len(), |(index, _)| index);
    Some(&text[..end])
}

#[cfg(test)]
mod tests {
    use super::{CommentKind, parse_directive_comment, parse_directive_text};

    #[test]
    fn parses_known_directive_kinds() {
        let cases = [
            "eslint-disable",
            "eslint-disable eqeqeq",
            "eslint-disable eqeqeq, no-console",
            "eslint-enable",
            "eslint-disable-line eqeqeq",
            "eslint-disable-next-line",
            "eslint-env node",
            "exported foo",
            "global foo",
            "globals foo",
            "eslint quotes: error",
        ];
        let parsed = cases.map(parse_directive_text);
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(parsed);
        }
    }

    #[test]
    fn rejects_non_directives() {
        let cases = ["", " ", "not-a-directive", "eslintfoo", "eslint-disablexyz"];
        let parsed = cases.map(parse_directive_text);
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(parsed);
        }
    }

    #[test]
    fn extracts_descriptions() {
        let cases = [
            "eslint-disable eqeqeq -- because reasons",
            "eslint-disable -- a -- b",
            "eslint-disable foo --- triple",
            "eslint-disable foo -short", // single hyphen is not a divider
        ];
        let parsed = cases.map(parse_directive_text);
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(parsed);
        }
    }

    #[test]
    fn honors_line_and_block_restrictions() {
        // Block-only kinds are not directives inside a line comment.
        let line_disable = parse_directive_comment(CommentKind::Line, "eslint-disable foo", true);
        // disable-line is fine in a line comment.
        let line_disable_line =
            parse_directive_comment(CommentKind::Line, "eslint-disable-line foo", true);
        // disable-line must stay on one line.
        let multiline_disable_line =
            parse_directive_comment(CommentKind::Block, "eslint-disable-line foo", false);
        // disable-next-line may span (block) lines.
        let multiline_next_line =
            parse_directive_comment(CommentKind::Block, "eslint-disable-next-line foo", false);
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!((
                line_disable,
                line_disable_line,
                multiline_disable_line,
                multiline_next_line,
            ));
        }
    }
}
