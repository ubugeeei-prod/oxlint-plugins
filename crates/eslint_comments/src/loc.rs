//! Source-location helpers shared by directive-comment rules.

/// A source position. `line` is 1-indexed; `column` is 0-indexed. A `column`
/// of `-1` is a sentinel meaning "force the whole line" (matching upstream
/// `toForceLocation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: i32,
}

/// A source range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub start: Position,
    pub end: Position,
}

/// Build a location that ignores the start column, used so diagnostics about a
/// directive comment sort to the start of the line. Mirrors upstream
/// `toForceLocation`.
pub fn to_force_location(location: Location) -> Location {
    Location {
        start: Position {
            line: location.start.line,
            column: -1,
        },
        end: location.end,
    }
}

/// `true` when `a` is at or before `b`. Mirrors upstream `lte`.
pub fn lte(a: Position, b: Position) -> bool {
    a.line < b.line || (a.line == b.line && a.column <= b.column)
}

/// `true` when the byte is a rule-id separator: ASCII whitespace or a comma
/// (the `[\s,]` character class in upstream's rule-id pattern).
fn is_rule_id_separator(byte: u8) -> bool {
    byte == b',' || byte.is_ascii_whitespace()
}

/// Find the byte offset of the first occurrence of `rule_id` in `line` that is
/// bounded by a separator (or the line edge) on both sides — the match of
/// upstream's `([\s,]|^)<ruleId>(?:[\s,]|$)` pattern.
fn find_rule_id(line: &str, rule_id: &str) -> Option<usize> {
    if rule_id.is_empty() {
        return None;
    }

    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(rel) = line[from..].find(rule_id) {
        let start = from + rel;
        let end = start + rule_id.len();
        let before_ok = start == 0 || is_rule_id_separator(bytes[start - 1]);
        let after_ok = end == line.len() || is_rule_id_separator(bytes[end]);
        if before_ok && after_ok {
            return Some(start);
        }
        // Advance past the matched occurrence by a whole character. `start + 1`
        // would land inside a multi-byte rule id (e.g. one starting with a
        // non-ASCII char), and the next `line[from..]` slice would panic on the
        // char boundary. `rule_id` is non-empty, so this always advances.
        from = start + line[start..].chars().next().map_or(1, char::len_utf8);
    }

    None
}

/// Compute the location of `rule_id` inside a directive comment. Mirrors
/// upstream `toRuleIdLocation`: when `rule_id` is `None` the whole line is
/// forced; otherwise the column points at the rule name within the comment.
///
/// `comment_value` is the comment body without its `//` or `/* */` delimiters;
/// `comment_loc.start.column` is the 0-indexed column of the opening delimiter.
/// Line splitting handles `\n` and `\r\n`; the rarer Unicode line separators
/// (` `/` `) are not split on.
pub fn to_rule_id_location(
    comment_value: &str,
    comment_loc: Location,
    rule_id: Option<&str>,
) -> Location {
    let Some(rule_id) = rule_id else {
        return to_force_location(comment_loc);
    };

    let start_line = comment_loc.start.line;
    for (index, raw_line) in comment_value.split('\n').enumerate() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let Some(pos) = find_rule_id(line, rule_id) else {
            continue;
        };

        // The opening delimiter (`//` or `/*`) only shifts the first line, since
        // later lines begin at column 0 in the source.
        let column = if index == 0 {
            2 + comment_loc.start.column + pos as i32
        } else {
            pos as i32
        };
        let line_number = start_line + index as u32;

        return Location {
            start: Position {
                line: line_number,
                column,
            },
            end: Position {
                line: line_number,
                column: column + rule_id.len() as i32,
            },
        };
    }

    comment_loc
}

#[cfg(test)]
mod tests {
    use super::{Location, Position, find_rule_id, to_rule_id_location};

    #[test]
    fn find_rule_id_matches_bounded_occurrence() {
        // The bounded standalone token is found, not the substring inside `xfoo`.
        assert_eq!(find_rule_id(" xfoo foo ", "foo"), Some(6));
        assert_eq!(find_rule_id("foo,bar", "bar"), Some(4));
        assert_eq!(find_rule_id("foobar", "foo"), None);
    }

    #[test]
    fn find_rule_id_handles_multibyte_without_panicking() {
        // A non-ASCII rule id that first appears inside a larger token used to
        // panic: advancing by one byte landed mid-character. The bounded later
        // occurrence must be found instead of crashing.
        let line = " xαfoo αfoo ";
        let found = find_rule_id(line, "αfoo");
        // The bounded standalone occurrence is the second one in the line.
        let expected = line.match_indices("αfoo").nth(1).map(|(index, _)| index);
        assert_eq!(found, expected);
        assert!(found.is_some());

        let loc = Location {
            start: Position { line: 1, column: 0 },
            end: Position {
                line: 1,
                column: 40,
            },
        };
        // Drives the full path; the assertion is mainly that it does not panic.
        let result = to_rule_id_location(line, loc, Some("αfoo"));
        assert_eq!(result.start.line, 1);
        assert!(result.start.column > 0);
    }
}
