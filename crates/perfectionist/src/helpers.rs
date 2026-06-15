//! Small string helpers shared across perfectionist sort-* rules.

use oxlint_plugins_carton::{CompactString, SmallVec};

pub(crate) fn find_matching(
    source_text: &str,
    open_index: usize,
    open: char,
    close: char,
) -> Option<usize> {
    let bytes = source_text.as_bytes();
    let mut cursor = open_index;
    let mut depth = 0usize;
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
            byte if byte == open as u8 => depth += 1,
            byte if byte == close as u8 => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

pub(crate) fn extract_names(segment: &str) -> SmallVec<[CompactString; 8]> {
    split_top_level(segment)
        .into_iter()
        .map(normalize_name)
        .filter(|name| !name.is_empty())
        .collect()
}

pub(crate) fn split_top_level(segment: &str) -> SmallVec<[&str; 8]> {
    let bytes = segment.as_bytes();
    let mut parts = SmallVec::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut start = 0usize;
    let mut cursor = 0usize;
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
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth = depth.saturating_sub(1),
            b',' | b';' if depth == 0 => {
                parts.push(segment[start..cursor].trim());
                start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    let tail = segment[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

pub(crate) fn normalize_name(item: &str) -> CompactString {
    let trimmed = item
        .trim()
        .trim_start_matches("case")
        .trim()
        .trim_matches(|ch| matches!(ch, '\'' | '"' | '`' | '[' | ']' | '(' | ')' | '{' | '}'));
    let end = trimmed
        .find(|ch: char| ch == ':' || ch == '=' || ch == '(' || ch.is_whitespace() || ch == ',')
        .unwrap_or(trimmed.len());
    let mut name = CompactString::from(trimmed[..end].trim());
    if name.as_str().starts_with("...") {
        name = CompactString::from(name.as_str().trim_start_matches("..."));
    }
    name
}

pub(crate) fn is_unsorted(names: &[CompactString]) -> bool {
    names
        .windows(2)
        .any(|pair| pair[0].as_str().to_ascii_lowercase() > pair[1].as_str().to_ascii_lowercase())
}
