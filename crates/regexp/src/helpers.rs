//! Small string-level helpers for parsing regexp patterns and flags.

use oxc_ast::ast::{ArrayExpressionElement, Expression};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

pub(crate) fn array_element_expression<'a>(
    element: &'a ArrayExpressionElement<'a>,
) -> Option<&'a Expression<'a>> {
    element.as_expression()
}

pub(crate) fn string_literal_value_with_span<'a>(
    expression: &'a Expression<'a>,
) -> Option<(&'a str, Span)> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Some((literal.value.as_str(), literal.span)),
        _ => None,
    }
}

pub(crate) fn duplicate_flag(flags: &str) -> Option<&str> {
    let mut seen = [false; 128];
    for (start, ch) in flags.char_indices() {
        let code = ch as usize;
        if code < seen.len() {
            if seen[code] {
                return Some(&flags[start..start + ch.len_utf8()]);
            }
            seen[code] = true;
        }
    }
    None
}

pub(crate) fn sorted_flags(flags: &str) -> CompactString {
    let mut chars = SmallVec::<[char; 8]>::new();
    chars.extend(flags.chars());
    chars.sort_unstable();
    let mut out = CompactString::new("");
    for ch in chars {
        out.push(ch);
    }
    out
}

pub(crate) fn skip_escape(bytes: &[u8], index: usize) -> usize {
    if index + 1 >= bytes.len() {
        return index + 1;
    }
    match bytes[index + 1] {
        b'u' if index + 2 < bytes.len() && bytes[index + 2] == b'{' => {
            let mut cursor = index + 3;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                cursor += 1;
            }
            cursor.saturating_add(1).min(bytes.len())
        }
        b'u' => (index + 6).min(bytes.len()),
        b'x' => (index + 4).min(bytes.len()),
        _ => (index + 2).min(bytes.len()),
    }
}

pub(crate) fn find_class_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut index = open + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = skip_escape(bytes, index),
            b']' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

/// Result of classifying the start of a `(` group: whether the body should be
/// checked for emptiness, whether the group captures, whether the group is a
/// named capture (`(?<name>...)`), and the byte index immediately after the
/// group prefix. Lookarounds and `?:` do not capture; anonymous `(...)` and
/// named `(?<name>...)` capture but only the latter is named.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GroupPrefix {
    pub(crate) check_empty: bool,
    pub(crate) capturing: bool,
    pub(crate) named: bool,
    pub(crate) is_lookaround: bool,
    pub(crate) next: usize,
}

pub(crate) fn group_prefix(bytes: &[u8], open: usize) -> GroupPrefix {
    if bytes.get(open + 1) != Some(&b'?') {
        return GroupPrefix {
            check_empty: true,
            capturing: true,
            named: false,
            is_lookaround: false,
            next: open + 1,
        };
    }
    match bytes.get(open + 2).copied() {
        Some(b':') => GroupPrefix {
            check_empty: true,
            capturing: false,
            named: false,
            is_lookaround: false,
            next: open + 3,
        },
        Some(b'=') | Some(b'!') => GroupPrefix {
            check_empty: false,
            capturing: false,
            named: false,
            is_lookaround: true,
            next: open + 3,
        },
        Some(b'<') => {
            if matches!(bytes.get(open + 3), Some(b'=') | Some(b'!')) {
                GroupPrefix {
                    check_empty: false,
                    capturing: false,
                    named: false,
                    is_lookaround: true,
                    next: open + 4,
                }
            } else {
                let mut cursor = open + 3;
                while cursor < bytes.len() && bytes[cursor] != b'>' {
                    cursor += 1;
                }
                GroupPrefix {
                    check_empty: true,
                    capturing: true,
                    named: true,
                    is_lookaround: false,
                    next: cursor.saturating_add(1).min(bytes.len()),
                }
            }
        }
        _ => GroupPrefix {
            check_empty: false,
            capturing: false,
            named: false,
            is_lookaround: false,
            next: open + 2,
        },
    }
}

/// Classification of a `[...]` character class for shorthand-equivalence
/// rules (`prefer-d`, `prefer-w`). Carries whether the class is negated so
/// callers can pick between the lower- and upper-case shorthand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ShorthandClass {
    pub(crate) negated: bool,
}

/// Returns `Some(ShorthandClass)` when the class at `open` is exactly the
/// range `0-9` (possibly negated): the bodies of `[0-9]`, `[^0-9]`. Any extra
/// element, malformed range, or unrecognised content yields `None`. Reuses
/// `find_class_end` so escaped `]` inside the class is handled.
pub(crate) fn class_is_digit_range(bytes: &[u8], open: usize) -> Option<ShorthandClass> {
    let (negated, start, end) = class_body_bounds(bytes, open)?;
    let body = &bytes[start..end];
    if body == b"0-9" {
        Some(ShorthandClass { negated })
    } else {
        None
    }
}

/// Returns `Some(ShorthandClass)` when the class at `open` is exactly the
/// word-character set in any order: ranges `a-z`, `A-Z`, `0-9`, and literal
/// `_`. Negated forms (`[^a-zA-Z0-9_]` etc.) are also recognised. Any extra
/// element or unrecognised content yields `None`.
pub(crate) fn class_is_word_char_set(bytes: &[u8], open: usize) -> Option<ShorthandClass> {
    let (negated, start, end) = class_body_bounds(bytes, open)?;
    let mut index = start;
    let mut saw_lower = false;
    let mut saw_upper = false;
    let mut saw_digit = false;
    let mut saw_underscore = false;
    while index < end {
        if index + 2 < end
            && bytes[index] == b'a'
            && bytes[index + 1] == b'-'
            && bytes[index + 2] == b'z'
            && !saw_lower
        {
            saw_lower = true;
            index += 3;
        } else if index + 2 < end
            && bytes[index] == b'A'
            && bytes[index + 1] == b'-'
            && bytes[index + 2] == b'Z'
            && !saw_upper
        {
            saw_upper = true;
            index += 3;
        } else if index + 2 < end
            && bytes[index] == b'0'
            && bytes[index + 1] == b'-'
            && bytes[index + 2] == b'9'
            && !saw_digit
        {
            saw_digit = true;
            index += 3;
        } else if bytes[index] == b'_' && !saw_underscore {
            saw_underscore = true;
            index += 1;
        } else {
            return None;
        }
    }
    if saw_lower && saw_upper && saw_digit && saw_underscore {
        Some(ShorthandClass { negated })
    } else {
        None
    }
}

/// Returns `(negated, body_start, body_end_exclusive)` for the `[...]` class at
/// `open`, where the body excludes the leading `[`, the optional negation `^`,
/// and the trailing `]`. Returns `None` if the class is unclosed.
fn class_body_bounds(bytes: &[u8], open: usize) -> Option<(bool, usize, usize)> {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let end = find_class_end(bytes, open)?;
    let mut start = open + 1;
    let mut negated = false;
    if bytes.get(start) == Some(&b'^') {
        negated = true;
        start += 1;
    }
    Some((negated, start, end))
}

/// Returns the first hexadecimal escape sequence (`\xHH`, `\uHHHH`, or
/// `\u{H+}`) in `pattern` whose hex digits contain at least one uppercase
/// letter `A`-`F`. Used by `letter-case` (default config: lowercase hex digits).
pub(crate) fn first_uppercase_hex_escape(pattern: &str) -> Option<&str> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }
        let next = *bytes.get(index + 1)?;
        match next {
            b'x' => {
                let end = (index + 4).min(bytes.len());
                if end - index == 4 && hex_range_has_upper(&bytes[index + 2..end]) {
                    return Some(&pattern[index..end]);
                }
                index = skip_escape(bytes, index);
            }
            b'u' if bytes.get(index + 2) == Some(&b'{') => {
                let mut cursor = index + 3;
                while cursor < bytes.len() && bytes[cursor] != b'}' {
                    cursor += 1;
                }
                if cursor < bytes.len() && hex_range_has_upper(&bytes[index + 3..cursor]) {
                    return Some(&pattern[index..cursor + 1]);
                }
                index = cursor.saturating_add(1).min(bytes.len());
            }
            b'u' => {
                let end = (index + 6).min(bytes.len());
                if end - index == 6 && hex_range_has_upper(&bytes[index + 2..end]) {
                    return Some(&pattern[index..end]);
                }
                index = skip_escape(bytes, index);
            }
            _ => index = skip_escape(bytes, index),
        }
    }
    None
}

fn hex_range_has_upper(bytes: &[u8]) -> bool {
    bytes.iter().any(|&byte| matches!(byte, b'A'..=b'F'))
}

/// Returns `true` when the `[...]` character class at `open` consists of
/// exactly an antipair of shorthand classes that together cover every
/// character: `[\s\S]`, `[\d\D]`, or `[\w\W]` (in either order). Returns
/// `false` for negated classes (`[^...]`), classes with extra elements, or
/// any unrecognised content. The check stops at the matching `]`; `\]` inside
/// the class is handled by reusing `find_class_end`.
pub(crate) fn class_matches_anything(bytes: &[u8], open: usize) -> bool {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let Some(end) = find_class_end(bytes, open) else {
        return false;
    };
    let mut index = open + 1;
    if bytes.get(index) == Some(&b'^') {
        return false;
    }
    let mut has_lower = [false; 3]; // s, d, w
    let mut has_upper = [false; 3]; // S, D, W
    let mut count = 0usize;
    while index < end {
        if count >= 2 {
            return false;
        }
        if bytes[index] != b'\\' {
            return false;
        }
        let Some(&kind) = bytes.get(index + 1) else {
            return false;
        };
        match kind {
            b's' => has_lower[0] = true,
            b'S' => has_upper[0] = true,
            b'd' => has_lower[1] = true,
            b'D' => has_upper[1] = true,
            b'w' => has_lower[2] = true,
            b'W' => has_upper[2] = true,
            _ => return false,
        }
        index += 2;
        count += 1;
    }
    (0..3).any(|i| has_lower[i] && has_upper[i])
}

/// Shape of a `{...}` braced quantifier that can be rewritten as a shorter
/// quantifier. `Plus` is `{1,}`, `Star` is `{0,}`, `Question` is `{0,1}`, and
/// `EqualTwoNums(n)` is `{n,n}` with `n >= 1` (the `n == 0` case is reported by
/// `no-zero-quantifier` instead).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BraceQuantifierShape {
    Plus,
    Star,
    Question,
    EqualTwoNums(u64),
}

/// Parse a `{n}`, `{n,}`, or `{n,m}` quantifier starting at `open` (the `{`
/// byte). Returns `(end_exclusive, original_text, shape)` if the quantifier is
/// well-formed and matches one of the four rewritable shapes; otherwise `None`.
/// The decision is intentionally conservative: malformed inputs (e.g. trailing
/// digits without a closing brace, non-digit characters) fall through to the
/// caller's existing scan loop.
pub(crate) fn parse_brace_quantifier(
    bytes: &[u8],
    open: usize,
) -> Option<(usize, &str, BraceQuantifierShape)> {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'{'));

    let mut cursor = open + 1;
    let first_start = cursor;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    if cursor == first_start {
        return None;
    }
    let first = parse_u64(&bytes[first_start..cursor])?;

    match bytes.get(cursor).copied() {
        Some(b'}') => {
            // `{n}` — never rewritable on its own (this is the canonical form).
            let _ = first;
            None
        }
        Some(b',') => {
            cursor += 1;
            let second_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if bytes.get(cursor) != Some(&b'}') {
                return None;
            }
            let end = cursor + 1;
            let original =
                std::str::from_utf8(&bytes[open..end]).expect("ASCII slice is valid UTF-8");

            if second_start == cursor {
                // `{n,}` — open-ended.
                let shape = match first {
                    0 => BraceQuantifierShape::Star,
                    1 => BraceQuantifierShape::Plus,
                    _ => return None,
                };
                Some((end, original, shape))
            } else {
                // `{n,m}`.
                let second = parse_u64(&bytes[second_start..cursor])?;
                if first == 0 && second == 1 {
                    Some((end, original, BraceQuantifierShape::Question))
                } else if first == second && first >= 1 {
                    Some((end, original, BraceQuantifierShape::EqualTwoNums(first)))
                } else {
                    None
                }
            }
        }
        _ => None,
    }
}

fn parse_u64(digits: &[u8]) -> Option<u64> {
    let mut value: u64 = 0;
    for &byte in digits {
        let digit = byte.checked_sub(b'0')?;
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u64::from(digit))?;
    }
    Some(value)
}

/// Returns `true` when the character class starting at `open` (a `[` byte)
/// contains at least one `\b` escape. The class is delimited by `find_class_end`
/// semantics, so `\]` inside the class is correctly skipped.
pub(crate) fn class_contains_backspace_escape(bytes: &[u8], open: usize) -> bool {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let Some(end) = find_class_end(bytes, open) else {
        return false;
    };

    let mut index = open + 1;
    while index < end {
        if bytes[index] == b'\\' {
            if bytes.get(index + 1) == Some(&b'b') {
                return true;
            }
            index = skip_escape(bytes, index).max(index + 1);
            continue;
        }
        index += 1;
    }
    false
}

pub(crate) fn is_zero_quantifier(bytes: &[u8], open: usize) -> bool {
    let mut cursor = open + 1;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    if cursor == open + 1 {
        return false;
    }
    let first = std::str::from_utf8(&bytes[open + 1..cursor]).unwrap_or("");
    if first != "0" {
        return false;
    }
    if bytes.get(cursor) == Some(&b'}') {
        return true;
    }
    if bytes.get(cursor) != Some(&b',') {
        return false;
    }
    cursor += 1;
    let second_start = cursor;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'}') {
        return false;
    }
    if cursor == second_start {
        return false;
    }
    std::str::from_utf8(&bytes[second_start..cursor]).unwrap_or("") == "0"
}

pub(crate) fn first_octal_escape(pattern: &str) -> Option<&str> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index + 2 < bytes.len() {
        if bytes[index] == b'\\'
            && bytes[index + 1] == b'0'
            && matches!(bytes[index + 2], b'0'..=b'7')
        {
            let mut end = index + 3;
            while end < bytes.len() && matches!(bytes[end], b'0'..=b'7') {
                end += 1;
            }
            return Some(&pattern[index..end]);
        }
        index = if bytes[index] == b'\\' {
            skip_escape(bytes, index)
        } else {
            index + 1
        };
    }
    None
}

/// Returns the first non-standard flag character in `flags` (i.e. one that is
/// not part of the canonical set `d`, `g`, `i`, `m`, `s`, `u`, `v`, `y`).
/// Used by `no-non-standard-flag`. ASCII-only by design; non-ASCII bytes are
/// also reported because they cannot be valid flags.
pub(crate) fn first_non_standard_flag(flags: &str) -> Option<char> {
    flags
        .chars()
        .find(|&ch| !matches!(ch, 'd' | 'g' | 'i' | 'm' | 's' | 'u' | 'v' | 'y'))
}

/// Returns the first literal invisible character in `pattern` that the
/// `no-invisible-character` rule recognises. Escaped sequences (`\u00A0`,
/// `\xA0`, `\u{1680}`) are intentionally skipped: the rule targets characters
/// that look like whitespace to a reader but are not the ASCII space, not
/// well-defined hex escapes.
pub(crate) fn first_invisible_character(pattern: &str) -> Option<char> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index);
            continue;
        }
        // Decode one UTF-8 scalar starting at `index`.
        let ch = pattern[index..].chars().next()?;
        if is_invisible_character(ch) {
            return Some(ch);
        }
        index += ch.len_utf8();
    }
    None
}

/// Curated set of "invisible" characters reported by `no-invisible-character`.
/// The set covers ECMAScript whitespace beyond U+0020 plus the zero-width
/// joiners, the line/paragraph separators, BOM, and the most common space
/// look-alikes that are commonly pasted accidentally.
fn is_invisible_character(ch: char) -> bool {
    matches!(
        ch,
        '\u{0009}'  // CHARACTER TABULATION (tab)
        | '\u{000B}' // LINE TABULATION (vertical tab)
        | '\u{000C}' // FORM FEED
        | '\u{0085}' // NEXT LINE
        | '\u{00A0}' // NO-BREAK SPACE
        | '\u{1680}' // OGHAM SPACE MARK
        | '\u{2000}'
            ..='\u{200A}' // various spaces
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
        | '\u{202F}' // NARROW NO-BREAK SPACE
        | '\u{205F}' // MEDIUM MATHEMATICAL SPACE
        | '\u{3000}' // IDEOGRAPHIC SPACE
        | '\u{200B}' // ZERO WIDTH SPACE
        | '\u{200C}' // ZERO WIDTH NON-JOINER
        | '\u{200D}' // ZERO WIDTH JOINER
        | '\u{FEFF}' // ZERO WIDTH NO-BREAK SPACE (BOM)
    )
}

/// Returns the first `\xHH` escape sequence in `pattern` together with its
/// `\u{HH}` replacement. Used by `hexadecimal-escape` (default config: flag
/// hex escapes and prefer unicode-style replacements). Other escapes such as
/// `\uHHHH`, `\u{H+}`, and `\d` are skipped via `skip_escape`.
pub(crate) fn first_hex_x_escape(pattern: &str) -> Option<(&str, CompactString)> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'x')
            && index + 4 <= bytes.len()
            && bytes[index + 2].is_ascii_hexdigit()
            && bytes[index + 3].is_ascii_hexdigit()
        {
            let original = &pattern[index..index + 4];
            let mut replacement = CompactString::new("\\u{");
            replacement.push(bytes[index + 2].to_ascii_lowercase() as char);
            replacement.push(bytes[index + 3].to_ascii_lowercase() as char);
            replacement.push('}');
            return Some((original, replacement));
        }
        index = skip_escape(bytes, index);
    }
    None
}

/// Returns the first fixed-width `\uHHHH` escape (i.e. not the `\u{H+}`
/// variant) in `pattern` together with its `\u{HHHH}` replacement. Used by
/// `unicode-escape` (default config: prefer the unicode code-point escape).
pub(crate) fn first_fixed_unicode_escape(pattern: &str) -> Option<(&str, CompactString)> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'u')
            && bytes.get(index + 2).copied() != Some(b'{')
            && index + 6 <= bytes.len()
            && bytes[index + 2].is_ascii_hexdigit()
            && bytes[index + 3].is_ascii_hexdigit()
            && bytes[index + 4].is_ascii_hexdigit()
            && bytes[index + 5].is_ascii_hexdigit()
        {
            let original = &pattern[index..index + 6];
            let mut replacement = CompactString::new("\\u{");
            for offset in 2..6 {
                replacement.push(bytes[index + offset].to_ascii_lowercase() as char);
            }
            replacement.push('}');
            return Some((original, replacement));
        }
        index = skip_escape(bytes, index);
    }
    None
}

/// Returns `Some(ch)` when the `[...]` class at `open` is exactly `[X]` for a
/// regular ASCII literal `X`. Negated classes, escaped contents, ranges,
/// nested classes, and bodies of length other than one are intentionally
/// ignored — they need more analysis to know the class is truly equivalent
/// to the bare character. Used by `no-useless-character-class`.
pub(crate) fn class_is_useless_single_literal(bytes: &[u8], open: usize) -> Option<char> {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let end = find_class_end(bytes, open)?;
    let start = open + 1;
    if bytes.get(start) == Some(&b'^') {
        return None;
    }
    if end - start != 1 {
        return None;
    }
    let byte = bytes[start];
    // Reject anything that would carry extra regex meaning by itself, anything
    // non-ASCII (multi-byte chars are fine in principle but reading them as a
    // single byte is incorrect), and anything that would change the surrounding
    // pattern if extracted from the class context.
    if !byte.is_ascii()
        || matches!(
            byte,
            b'\\'
                | b'-'
                | b'['
                | b']'
                | b'^'
                | b'$'
                | b'.'
                | b'|'
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b'?'
                | b'{'
                | b'}'
        )
    {
        return None;
    }
    Some(byte as char)
}

/// Returns `true` when `pattern` contains the literal sequence `\q{}` — an
/// empty string literal inside a class. The `\q{...}` syntax is only valid in
/// `v`-flag patterns, so its presence implies `v`-mode without us needing to
/// inspect the flags here. Used by `no-empty-string-literal`.
pub(crate) fn pattern_has_empty_string_literal(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if bytes[index] == b'\\'
            && bytes[index + 1] == b'q'
            && bytes[index + 2] == b'{'
            && bytes[index + 3] == b'}'
        {
            return true;
        }
        index += 1;
    }
    false
}

/// Returns `true` when the `[...]` class at `open` contains at least one
/// `X-X` range whose start and end byte are literal ASCII characters that
/// match exactly. Used by `no-useless-range`. Escaped or compound ranges
/// (`\d-\d`, `\xHH-\xHH`) are intentionally not handled — they need
/// equivalence analysis that we defer.
pub(crate) fn class_has_useless_range(bytes: &[u8], open: usize) -> Option<char> {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let end = find_class_end(bytes, open)?;
    let mut index = open + 1;
    if bytes.get(index) == Some(&b'^') {
        index += 1;
    }
    while index < end {
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index).min(end);
            continue;
        }
        if index + 2 < end
            && bytes[index + 1] == b'-'
            && bytes[index + 2] != b'\\'
            && bytes[index + 2] != b']'
            && bytes[index] == bytes[index + 2]
        {
            return Some(bytes[index] as char);
        }
        index += 1;
    }
    None
}

pub(crate) fn first_control_character(pattern: &str) -> Option<char> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            if let Some(ch) = escaped_control_character(bytes, index) {
                return Some(ch);
            }
            index = skip_escape(bytes, index);
            continue;
        }

        let Some(ch) = pattern[index..].chars().next() else {
            break;
        };
        if ch <= '\u{1f}' {
            return Some(ch);
        }
        index += ch.len_utf8();
    }
    None
}

fn escaped_control_character(bytes: &[u8], index: usize) -> Option<char> {
    let code = match bytes.get(index + 1).copied()? {
        b'x' if index + 3 < bytes.len() => {
            u32::from((hex_value(bytes[index + 2])? << 4) | hex_value(bytes[index + 3])?)
        }
        b'u' if index + 2 < bytes.len() && bytes[index + 2] == b'{' => {
            let mut cursor = index + 3;
            let mut value = 0u32;
            let mut saw_digit = false;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                value = (value << 4) | u32::from(hex_value(bytes[cursor])?);
                saw_digit = true;
                cursor += 1;
            }
            if !saw_digit || bytes.get(cursor) != Some(&b'}') {
                return None;
            }
            value
        }
        b'u' if index + 5 < bytes.len() => {
            let mut value = 0u32;
            for byte in &bytes[index + 2..index + 6] {
                value = (value << 4) | u32::from(hex_value(*byte)?);
            }
            value
        }
        _ => return None,
    };
    if code <= 0x1f {
        char::from_u32(code)
    } else {
        None
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn mention_char(ch: char) -> CompactString {
    let mut text = CompactString::new("U+");
    let code = ch as u32;
    let mut buf = [0u8; 6];
    let mut value = code;
    let mut cursor = buf.len();
    if value == 0 {
        cursor -= 1;
        buf[cursor] = b'0';
    } else {
        while value > 0 {
            cursor -= 1;
            let digit = (value & 0xf) as u8;
            buf[cursor] = if digit < 10 {
                b'0' + digit
            } else {
                b'A' + (digit - 10)
            };
            value >>= 4;
        }
    }
    for _ in 0..(4usize.saturating_sub(buf.len() - cursor)) {
        text.push('0');
    }
    if let Ok(hex) = std::str::from_utf8(&buf[cursor..]) {
        text.push_str(hex);
    }
    text
}
