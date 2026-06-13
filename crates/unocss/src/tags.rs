//! Lightweight tag/attribute walker for the `order-attributify` rule.

pub(crate) const IGNORED_ATTRIBUTIFY_ATTRIBUTES: [&str; 4] =
    ["style", "class", "classname", "value"];

pub(crate) fn find_tag_end(source_text: &str, mut cursor: usize) -> Option<usize> {
    let bytes = source_text.as_bytes();
    let mut quote = None;
    while cursor < bytes.len() {
        if let Some(active_quote) = quote {
            if bytes[cursor] == b'\\' {
                cursor = (cursor + 2).min(bytes.len());
                continue;
            }
            if bytes[cursor] == active_quote {
                quote = None;
            }
            cursor += 1;
            continue;
        }
        match bytes[cursor] {
            b'\'' | b'"' | b'`' => quote = Some(bytes[cursor]),
            b'>' => return Some(cursor),
            _ => {}
        }
        cursor += 1;
    }
    None
}

pub(crate) fn skip_attribute_value(source_text: &str, mut cursor: usize, tag_end: usize) -> usize {
    let bytes = source_text.as_bytes();
    while cursor < tag_end && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor >= tag_end {
        return cursor;
    }
    if matches!(bytes[cursor], b'\'' | b'"' | b'`') {
        let quote = bytes[cursor];
        cursor += 1;
        while cursor < tag_end {
            if bytes[cursor] == b'\\' {
                cursor = (cursor + 2).min(tag_end);
                continue;
            }
            if bytes[cursor] == quote {
                return cursor + 1;
            }
            cursor += 1;
        }
        return cursor;
    }
    if bytes[cursor] == b'{' {
        let mut depth = 1usize;
        cursor += 1;
        while cursor < tag_end && depth > 0 {
            match bytes[cursor] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            cursor += 1;
        }
        return cursor;
    }
    while cursor < tag_end && !bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    cursor
}

pub(crate) fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$'
}

pub(crate) fn is_identifier_part(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit() || byte == b'-'
}

pub(crate) fn is_attr_name_part(byte: u8) -> bool {
    is_identifier_part(byte) || byte == b':' || byte == b'.'
}
