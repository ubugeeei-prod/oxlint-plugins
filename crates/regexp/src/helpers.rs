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
        // v-mode string disjunction: \q{...} — skip past the closing `}`.
        b'q' if index + 2 < bytes.len() && bytes[index + 2] == b'{' => {
            let mut cursor = index + 3;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                cursor += 1;
            }
            cursor.saturating_add(1).min(bytes.len())
        }
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
    pub(crate) is_non_capturing: bool,
    pub(crate) next: usize,
}

pub(crate) fn group_prefix(bytes: &[u8], open: usize) -> GroupPrefix {
    if bytes.get(open + 1) != Some(&b'?') {
        return GroupPrefix {
            check_empty: true,
            capturing: true,
            named: false,
            is_lookaround: false,
            is_non_capturing: false,
            next: open + 1,
        };
    }
    match bytes.get(open + 2).copied() {
        Some(b':') => GroupPrefix {
            check_empty: true,
            capturing: false,
            named: false,
            is_lookaround: false,
            is_non_capturing: true,
            next: open + 3,
        },
        Some(b'=') | Some(b'!') => GroupPrefix {
            check_empty: false,
            capturing: false,
            named: false,
            is_lookaround: true,
            is_non_capturing: false,
            next: open + 3,
        },
        Some(b'<') => {
            if matches!(bytes.get(open + 3), Some(b'=') | Some(b'!')) {
                GroupPrefix {
                    check_empty: false,
                    capturing: false,
                    named: false,
                    is_lookaround: true,
                    is_non_capturing: false,
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
                    is_non_capturing: false,
                    next: cursor.saturating_add(1).min(bytes.len()),
                }
            }
        }
        _ => GroupPrefix {
            check_empty: false,
            capturing: false,
            named: false,
            is_lookaround: false,
            is_non_capturing: false,
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
    // `[\s\S]` is the canonical form the rule itself recommends — treat it as valid.
    let body = &bytes[open + 1..end];
    if body == b"\\s\\S" {
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

/// Returns the first `\uHHHH` or `\u{H+}` escape sequence in `pattern` whose
/// decoded code point is ≤ 0xFF, together with its `\xHH` replacement. Used by
/// `hexadecimal-escape` (default `"always"` config: flag unicode escapes that
/// can be written as `\xHH` and suggest the hexadecimal form). Code points
/// above 0xFF are not representable as `\xHH` and are silently skipped.
/// `\xHH` escapes (already in the correct form) are also skipped.
pub(crate) fn first_unicode_escape_as_hex(pattern: &str) -> Option<(&str, CompactString)> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'u') {
            if bytes.get(index + 2) == Some(&b'{') {
                // \u{H+} form
                let mut cursor = index + 3;
                while cursor < bytes.len() && bytes[cursor] != b'}' {
                    cursor += 1;
                }
                if cursor < bytes.len() {
                    let hex_str = &pattern[index + 3..cursor];
                    if let Ok(code_point) = u32::from_str_radix(hex_str, 16) {
                        if code_point <= 0xFF {
                            let original = &pattern[index..cursor + 1];
                            let replacement = format!("\\x{:02x}", code_point);
                            return Some((original, CompactString::from(replacement.as_str())));
                        }
                    }
                    index = cursor + 1;
                    continue;
                }
                index = cursor.saturating_add(1).min(bytes.len());
                continue;
            }
            // \uHHHH form (fixed 4 hex digits)
            if index + 6 <= bytes.len()
                && bytes[index + 2].is_ascii_hexdigit()
                && bytes[index + 3].is_ascii_hexdigit()
                && bytes[index + 4].is_ascii_hexdigit()
                && bytes[index + 5].is_ascii_hexdigit()
            {
                let hex_str = &pattern[index + 2..index + 6];
                if let Ok(code_point) = u32::from_str_radix(hex_str, 16) {
                    if code_point <= 0xFF {
                        let original = &pattern[index..index + 6];
                        let replacement = format!("\\x{:02x}", code_point);
                        return Some((original, CompactString::from(replacement.as_str())));
                    }
                }
            }
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
            // Surrogate halves (U+D800..=U+DFFF) belong to surrogate pairs handled
            // by `prefer-unicode-codepoint-escapes`; skip them here.
            let value = read_fixed_hex4(&bytes[index + 2..index + 6]).unwrap_or(0);
            if (0xD800..=0xDFFF).contains(&value) {
                index = skip_escape(bytes, index);
                continue;
            }
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

/// Returns `Some((start, end))` when the `[...]` class at `open` contains at
/// least one `X-Y` range whose endpoints cross ASCII character categories
/// (digit/uppercase letter/lowercase letter/other). Such ranges almost always
/// sweep up unexpected characters in the gaps between categories (the
/// canonical example is `[A-z]`, which includes `[\]^_` and `` ` ``). Used by
/// `no-obscure-range`. Escaped or compound endpoints are intentionally
/// ignored so the check stays sound (no false positives).
pub(crate) fn class_first_obscure_range(bytes: &[u8], open: usize) -> Option<(char, char)> {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let end = find_class_end(bytes, open)?;
    let mut index = open + 1;
    if bytes.get(index) == Some(&b'^') {
        index += 1;
    }
    while index + 2 < end {
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index).min(end);
            continue;
        }
        if bytes[index + 1] != b'-' {
            index += 1;
            continue;
        }
        if bytes[index + 2] == b'\\' || bytes[index + 2] == b']' {
            // Skip escaped or end-of-class endpoints; equivalence is hard to
            // judge without decoding the escape.
            index += 1;
            continue;
        }
        let start_byte = bytes[index];
        let end_byte = bytes[index + 2];
        if start_byte.is_ascii() && end_byte.is_ascii() && is_obscure_range(start_byte, end_byte) {
            return Some((start_byte as char, end_byte as char));
        }
        index += 3;
    }
    None
}

fn is_obscure_range(start: u8, end: u8) -> bool {
    if start > end {
        return false;
    }
    let start_category = ascii_category(start);
    let end_category = ascii_category(end);
    // A range stays within one of the canonical categories (digits or
    // lowercase letters or uppercase letters) — that is fine. Anything else
    // crosses a boundary and is flagged.
    !matches!(
        (start_category, end_category),
        (AsciiCategory::Digit, AsciiCategory::Digit)
            | (AsciiCategory::Lowercase, AsciiCategory::Lowercase)
            | (AsciiCategory::Uppercase, AsciiCategory::Uppercase)
    ) && (start_category != AsciiCategory::Other || end_category != AsciiCategory::Other)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AsciiCategory {
    Digit,
    Lowercase,
    Uppercase,
    Other,
}

fn ascii_category(byte: u8) -> AsciiCategory {
    match byte {
        b'0'..=b'9' => AsciiCategory::Digit,
        b'a'..=b'z' => AsciiCategory::Lowercase,
        b'A'..=b'Z' => AsciiCategory::Uppercase,
        _ => AsciiCategory::Other,
    }
}

/// Returns the first surrogate-pair escape sequence `\uHHHH\uHHHH` in
/// `pattern` together with its `\u{CODEPOINT}` replacement (hex digits in
/// lower case). Used by `prefer-unicode-codepoint-escapes`. Unrelated
/// adjacent `\uHHHH` escapes (where the pair is not a valid surrogate pair)
/// are skipped.
pub(crate) fn first_surrogate_pair_escape(pattern: &str) -> Option<(&str, CompactString)> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index + 12 <= bytes.len() {
        if bytes[index] != b'\\' || bytes[index + 1] != b'u' {
            index += 1;
            continue;
        }
        let Some(high) = read_fixed_hex4(&bytes[index + 2..index + 6]) else {
            index = skip_escape(bytes, index);
            continue;
        };
        if !(0xD800..=0xDBFF).contains(&high) {
            index = skip_escape(bytes, index);
            continue;
        }
        if bytes[index + 6] != b'\\' || bytes[index + 7] != b'u' {
            index = skip_escape(bytes, index);
            continue;
        }
        let Some(low) = read_fixed_hex4(&bytes[index + 8..index + 12]) else {
            index = skip_escape(bytes, index);
            continue;
        };
        if !(0xDC00..=0xDFFF).contains(&low) {
            index = skip_escape(bytes, index);
            continue;
        }
        let original = &pattern[index..index + 12];
        let codepoint = ((high - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
        let mut replacement = CompactString::new("\\u{");
        append_lower_hex(&mut replacement, codepoint);
        replacement.push('}');
        return Some((original, replacement));
    }
    None
}

fn read_fixed_hex4(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 4 {
        return None;
    }
    let mut value: u32 = 0;
    for byte in bytes.iter().take(4) {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return None,
        };
        value = value * 16 + u32::from(digit);
    }
    Some(value)
}

fn append_lower_hex(target: &mut CompactString, mut value: u32) {
    if value == 0 {
        target.push('0');
        return;
    }
    let mut buf = [0u8; 8];
    let mut cursor = buf.len();
    while value > 0 {
        cursor -= 1;
        let digit = (value & 0xf) as u8;
        buf[cursor] = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        };
        value >>= 4;
    }
    if let Ok(text) = std::str::from_utf8(&buf[cursor..]) {
        target.push_str(text);
    }
}

/// Returns `Some(original)` when the pattern contains a literal `{1}` or
/// `{1,1}` quantifier at the top level. Such quantifiers are no-ops; the
/// referenced atom matches itself exactly once anyway. Class context is
/// skipped because `{` and `1` are literal characters inside `[...]`. Used by
/// `no-useless-quantifier`.
pub(crate) fn first_useless_one_quantifier(pattern: &str) -> Option<&str> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'[' {
            if let Some(close) = find_class_end(bytes, index) {
                index = close + 1;
                continue;
            }
            return None;
        }
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index).max(index + 1);
            continue;
        }
        if bytes[index] == b'{' && bytes.get(index + 1) == Some(&b'1') {
            if bytes.get(index + 2) == Some(&b'}') {
                return Some(&pattern[index..index + 3]);
            }
            if bytes.get(index + 2) == Some(&b',')
                && bytes.get(index + 3) == Some(&b'1')
                && bytes.get(index + 4) == Some(&b'}')
            {
                return Some(&pattern[index..index + 5]);
            }
        }
        index += 1;
    }
    None
}

/// Returns `true` when the pattern ends with a lazy quantifier (`*?`, `+?`,
/// `??`, or `{...}?`) and nothing after it. A lazy quantifier at the very end
/// of a pattern always prefers to match as little as possible, which usually
/// means matching nothing — the quantifier is effectively dead code. Used by
/// `no-lazy-ends`. The check is purely textual to stay conservative; anchored
/// patterns like `a*?$` where the `$` follows are excluded by definition.
pub(crate) fn pattern_ends_with_lazy_quantifier(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    if len < 2 {
        return false;
    }
    // The last byte must be the lazy `?`.
    if bytes[len - 1] != b'?' {
        return false;
    }
    let preceding = bytes[len - 2];
    if matches!(preceding, b'*' | b'+' | b'?') {
        // Make sure the quantifier byte itself is not an escape (e.g. `\*?`).
        if len >= 3 && bytes[len - 3] == b'\\' {
            // Need to count backslashes to know whether the quantifier is escaped:
            // `\\*?` (backslash escape of `\`, then `*?`) is a quantifier; `\*?`
            // is an escaped `*` followed by `?`. Walk leading backslashes.
            let mut count = 0;
            let mut idx = len - 3;
            while idx > 0 && bytes[idx] == b'\\' {
                count += 1;
                idx -= 1;
            }
            // Plus the one we already saw.
            if count % 2 == 0 {
                return true;
            }
            return false;
        }
        return true;
    }
    if preceding == b'}' {
        // Walk back to find the matching `{` and verify it is a braced quantifier.
        let mut idx = len - 3;
        while idx > 0 && bytes[idx] != b'{' && bytes[idx] != b']' {
            idx -= 1;
        }
        if bytes.get(idx) == Some(&b'{')
            && (idx == 0 || bytes[idx - 1] != b'\\')
            && bytes[idx + 1..len - 2]
                .iter()
                .all(|&b| b.is_ascii_digit() || b == b',')
        {
            return true;
        }
    }
    false
}

/// Returns `Some(text)` for the first numbered backreference `\N` (N in 1-9)
/// inside `pattern` where capture group N is itself a named capture group
/// `(?<name>...)`. Only named-group backreferences have a `\k<name>` alternative,
/// so `\N` referring to an unnamed group must not be flagged. Backreferences
/// inside character classes are skipped because they are literal characters there.
pub(crate) fn first_numbered_backreference_with_named_group(pattern: &str) -> Option<&str> {
    let bytes = pattern.as_bytes();
    // Fast path: no named group syntax at all.
    if !pattern.contains("(?<") {
        return None;
    }

    // Pass 1: walk the pattern and record which 1-based capture group indices
    // are named.  A `(` that is NOT `(?:`, `(?=`, `(?!`, `(?<=`, `(?<!`
    // increments the group counter; `(?<name>...)` marks that index as named.
    // We use a u32 bitmask (bits 1-9 = groups 1-9) because the rule only
    // checks single-digit backreferences \1..\9.
    let mut named_mask: u32 = 0;
    {
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
                // Classify the group via the existing helper.
                let gp = group_prefix(bytes, index);
                if gp.capturing {
                    group_counter += 1;
                    if gp.named && group_counter <= 9 {
                        named_mask |= 1 << group_counter;
                    }
                }
                // Advance past the group prefix (the helper already skipped
                // the opening `(` plus any `?<name>` prefix).
                index = gp.next;
                continue;
            }
            index += 1;
        }
    }

    if named_mask == 0 {
        return None;
    }

    // Pass 2: find the first \N where group N is named.
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'[' {
            if let Some(close) = find_class_end(bytes, index) {
                index = close + 1;
                continue;
            }
            return None;
        }
        if bytes[index] == b'\\'
            && let Some(&next) = bytes.get(index + 1)
            && matches!(next, b'1'..=b'9')
        {
            let group_num = (next - b'0') as u32;
            if named_mask & (1 << group_num) != 0 {
                return Some(&pattern[index..index + 2]);
            }
            // Not a named group — skip past this escape and continue.
            index = skip_escape(bytes, index).max(index + 1);
            continue;
        }
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index).max(index + 1);
            continue;
        }
        index += 1;
    }
    None
}

/// Returns `Some(byte)` for the first escape `\X` in `pattern` where `X` is
/// a character that is never special in a regular expression (not in a
/// character class either), so the `\` is useless. Stays narrow on purpose:
/// only flags a curated list of punctuation that has no escape semantics
/// (`:`, `;`, `,`, `=`, `!`, `#`, `@`, `<`, `>`, `&`, `_`, `%`, `~`, `'`,
/// `"`, `/`). Walks the pattern with the existing class-skipping logic so
/// escapes inside `[...]` (where `]` and `-` carry extra meaning) are not
/// considered. Used by `no-useless-escape`.
pub(crate) fn first_useless_escape(pattern: &str) -> Option<u8> {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'[' {
            // Skip the entire character class — the rules for "useless" inside
            // a class differ, and we conservatively defer them.
            if let Some(close) = find_class_end(bytes, index) {
                index = close + 1;
                continue;
            }
            return None;
        }
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }
        if let Some(&next) = bytes.get(index + 1)
            && is_pointlessly_escaped(next)
        {
            return Some(next);
        }
        index = skip_escape(bytes, index).max(index + 1);
    }
    None
}

// Note: `/` is intentionally absent. Escaping the forward slash in a regex
// literal (`/\//`) is necessary — an unescaped `/` would terminate the literal
// — so upstream `no-useless-escape` treats `\/` as a required, non-useless
// escape.
fn is_pointlessly_escaped(byte: u8) -> bool {
    matches!(
        byte,
        b':' | b';'
            | b','
            | b'='
            | b'!'
            | b'#'
            | b'@'
            | b'<'
            | b'>'
            | b'&'
            | b'_'
            | b'%'
            | b'~'
            | b'\''
            | b'"'
    )
}

/// Returns `true` when the `[...]` class at `open` contains both the lower-
/// and upper-case form of at least one ASCII letter (e.g. `[aA]` or
/// `[abcABC]`). Such pairs could be expressed more concisely with the `i`
/// flag, which is what `use-ignore-case` recommends. Escaped contents and
/// ranges are deliberately skipped so the check stays sound (no false
/// positives on `[\w]` or `[a-z]`).
pub(crate) fn class_has_case_pair(bytes: &[u8], open: usize) -> bool {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let Some(end) = find_class_end(bytes, open) else {
        return false;
    };
    let mut index = open + 1;
    if bytes.get(index) == Some(&b'^') {
        index += 1;
    }
    let mut has_lower = [false; 26];
    let mut has_upper = [false; 26];
    while index < end {
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index).min(end);
            continue;
        }
        if index + 2 < end && bytes[index + 1] == b'-' {
            index += 3;
            continue;
        }
        let byte = bytes[index];
        if byte.is_ascii_lowercase() {
            has_lower[(byte - b'a') as usize] = true;
        } else if byte.is_ascii_uppercase() {
            has_upper[(byte - b'A') as usize] = true;
        }
        index += 1;
    }
    (0..26).any(|i| has_lower[i] && has_upper[i])
}

/// Returns `Some(byte)` when the `[...]` class at `open` contains a `\q{X}`
/// string literal whose body is exactly one ASCII char. Such literals are
/// equivalent to the bare character in v-mode, so the `\q{}` wrapper is
/// useless. Used by `grapheme-string-literal`. Multi-character string
/// literals (`\q{ab}`, `\q{}`) and non-ASCII bodies are deferred — the bare
/// equivalent depends on grapheme analysis we have not implemented.
///
/// Returns `true` when the `[...]` class at `open` is composed exclusively of
/// distinct ASCII alphanumeric byte literals AND those bytes appear out of
/// sorted order. Classes containing escapes, ranges, the `^` negation, or
/// non-alphanumeric literals are deferred (the sort order semantics get
/// hairy across categories). Used by `sort-character-class-elements`.
pub(crate) fn class_has_unsorted_literal_elements(bytes: &[u8], open: usize) -> bool {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let Some(end) = find_class_end(bytes, open) else {
        return false;
    };
    let mut index = open + 1;
    if bytes.get(index) == Some(&b'^') {
        return false;
    }
    let mut prev: u8 = 0;
    let mut unsorted = false;
    while index < end {
        let byte = bytes[index];
        if byte == b'\\' {
            return false;
        }
        if index + 2 < end && bytes[index + 1] == b'-' {
            return false;
        }
        if !byte.is_ascii_alphanumeric() {
            return false;
        }
        if byte < prev {
            unsorted = true;
        }
        prev = byte;
        index += 1;
    }
    unsorted
}

pub(crate) fn class_has_useless_string_literal(bytes: &[u8], open: usize) -> Option<u8> {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let end = find_class_end(bytes, open)?;
    let mut index = open + 1;
    while index + 4 < end {
        if bytes[index] == b'\\' && bytes[index + 1] == b'q' && bytes[index + 2] == b'{' {
            let mut cursor = index + 3;
            while cursor < end && bytes[cursor] != b'}' {
                cursor += 1;
            }
            if cursor < end {
                let inner = &bytes[index + 3..cursor];
                if inner.len() == 1 {
                    let ch = inner[0];
                    if ch.is_ascii_alphanumeric() {
                        return Some(ch);
                    }
                }
                index = cursor + 1;
                continue;
            }
        }
        index += 1;
    }
    None
}

/// Returns `Some(byte)` for the first ASCII literal byte that appears more
/// than once in the `[...]` class at `open`. Escapes, ranges, and nested
/// classes are intentionally skipped — comparing them for equivalence needs
/// decoding we have not implemented yet. Used by
/// `no-dupe-characters-character-class`. Keeps a small fixed-size bitmap on
/// the stack so the check has no allocation.
pub(crate) fn class_first_duplicate_literal(bytes: &[u8], open: usize) -> Option<u8> {
    debug_assert_eq!(bytes.get(open).copied(), Some(b'['));
    let end = find_class_end(bytes, open)?;
    let mut index = open + 1;
    if bytes.get(index) == Some(&b'^') {
        index += 1;
    }
    // 16 bytes covers all 128 ASCII bit slots.
    let mut seen = [0u8; 16];
    while index < end {
        if bytes[index] == b'\\' {
            index = skip_escape(bytes, index).min(end);
            continue;
        }
        if index + 2 < end && bytes[index + 1] == b'-' {
            // Skip the entire range — `a-c` does not mean three repeats of
            // any character, and the helper deliberately ignores ranges.
            index += 3;
            continue;
        }
        let byte = bytes[index];
        if byte.is_ascii() {
            let bit = byte as usize;
            let slot = bit / 8;
            let mask = 1u8 << (bit % 8);
            if seen[slot] & mask != 0 {
                return Some(byte);
            }
            seen[slot] |= mask;
        }
        index += 1;
    }
    None
}

/// Returns `Some((start, end))` when the `[...]` class at `open` contains
/// four or more consecutive ASCII characters (digits, lower-case, or
/// upper-case letters) at the top level — these collapse into the equivalent
/// range `start-end`. Used by `prefer-range`. A run of three (e.g. `[abc]`)
/// is left alone to match upstream, which only collapses runs of four or
/// more. Escapes and existing ranges are intentionally skipped to keep the
/// check conservative.
pub(crate) fn class_first_collapsible_run(bytes: &[u8], open: usize) -> Option<(char, char)> {
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
        if index + 2 < end && bytes[index + 1] == b'-' {
            index += 3;
            continue;
        }
        if is_collapsible_run_byte(bytes[index]) {
            let start = bytes[index];
            let mut run_end = start;
            let mut cursor = index + 1;
            while cursor < end
                && bytes[cursor] == run_end + 1
                && is_collapsible_run_byte(bytes[cursor])
            {
                run_end = bytes[cursor];
                cursor += 1;
            }
            if run_end >= start + 3 {
                return Some((start as char, run_end as char));
            }
            index = cursor.max(index + 1);
            continue;
        }
        index += 1;
    }
    None
}

fn is_collapsible_run_byte(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z')
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
    // `=` is also excluded: upstream exempts `[=]` because in some legacy regex
    // flavours `[=X=]` is a POSIX equivalence class; keeping the brackets avoids
    // accidental meaning changes and upstream explicitly allows it.
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
                | b'='
        )
    {
        return None;
    }
    Some(byte as char)
}

/// Returns `true` when removing the `[...]` brackets around the single
/// character at `open` would change the meaning of the surrounding pattern.
/// This guards `no-useless-character-class` from false positives in cases
/// where the class character is syntactically significant in context:
///
/// - `\c[X]` — removing brackets produces `\cX` (a control-character escape).
/// - `\xH[X]` — removing brackets completes a `\xHX` hex escape.
/// - `\uH[X]` — removing brackets supplies the second digit of a `\uHXXX` unicode escape.
/// - `\N[D]` (N = 1–9, D = digit) — removing brackets makes `\ND`, a multi-digit
///   back-reference that refers to a different group.
/// - `\0[D]` (D = octal digit 0–7) — removing brackets makes `\0D`, an octal escape.
/// - `{digits[D]` — the bracket is inside a `{n}` quantifier body; removing
///   brackets makes the quantifier literal (e.g. `a{[0]}` → `a{0}`).
///
/// `class_char` is the single byte inside `[...]` (already validated as ASCII
/// and not inherently special by `class_is_useless_single_literal`).
pub(crate) fn class_bracket_changes_meaning(bytes: &[u8], open: usize, class_char: u8) -> bool {
    // \c[X] → \cX control-character escape (X must be an ASCII letter).
    if open >= 2
        && bytes[open - 1] == b'c'
        && bytes[open - 2] == b'\\'
        && class_char.is_ascii_alphabetic()
    {
        return true;
    }
    // \xH[X] → \xHX or \uH[X] → \uHX... — completing a hex/unicode escape.
    if open >= 3
        && bytes[open - 3] == b'\\'
        && matches!(bytes[open - 2], b'x' | b'u')
        && bytes[open - 1].is_ascii_hexdigit()
        && class_char.is_ascii_hexdigit()
    {
        return true;
    }
    // \N[D] (N in 1–9, D a digit) → \ND multi-digit back-reference.
    if open >= 2
        && bytes[open - 2] == b'\\'
        && matches!(bytes[open - 1], b'1'..=b'9')
        && class_char.is_ascii_digit()
    {
        return true;
    }
    // \0[D] (D an octal digit 0–7) → \0D octal escape.
    if open >= 2
        && bytes[open - 2] == b'\\'
        && bytes[open - 1] == b'0'
        && matches!(class_char, b'0'..=b'7')
    {
        return true;
    }
    // {digits[D] — bracket is inside a `{n}` quantifier body.
    // Walk backwards over ASCII digits; if we reach an unescaped `{` the
    // bracket is inside a quantifier and removing it would change the meaning.
    let mut p = open;
    while p > 0 {
        p -= 1;
        if bytes[p].is_ascii_digit() {
            continue;
        }
        if bytes[p] == b'{' && !(p > 0 && bytes[p - 1] == b'\\') {
            return true;
        }
        break;
    }
    false
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

/// Returns the first raw control character (`U+0000`-`U+001F` or `U+007F`)
/// that appears literally in `pattern`. Unlike `first_control_character` this
/// helper does NOT decode escape sequences — `\x01` as a four-character escape
/// is returned `None` because the control character is not present in the
/// pattern bytes themselves. Used by `control-character-escape`, which targets
/// only the literal occurrences and asks the author to use an escape form.
pub(crate) fn first_literal_control_character(pattern: &str) -> Option<char> {
    for ch in pattern.chars() {
        let code = ch as u32;
        if code < 0x20 || code == 0x7f {
            return Some(ch);
        }
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
