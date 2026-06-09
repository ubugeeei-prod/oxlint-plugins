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
        from = start + 1;
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
