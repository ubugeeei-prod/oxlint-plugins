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

pub(crate) fn group_prefix(bytes: &[u8], open: usize) -> (bool, bool, usize) {
    if bytes.get(open + 1) != Some(&b'?') {
        return (true, true, open + 1);
    }
    match bytes.get(open + 2).copied() {
        Some(b':') => (true, false, open + 3),
        Some(b'=') | Some(b'!') => (false, false, open + 3),
        Some(b'<') => {
            if matches!(bytes.get(open + 3), Some(b'=') | Some(b'!')) {
                (false, false, open + 4)
            } else {
                let mut cursor = open + 3;
                while cursor < bytes.len() && bytes[cursor] != b'>' {
                    cursor += 1;
                }
                (true, true, cursor.saturating_add(1).min(bytes.len()))
            }
        }
        _ => (false, false, open + 2),
    }
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
