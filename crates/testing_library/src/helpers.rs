//! Small text helpers shared across testing-library scanner methods.

use oxc_span::Span;

pub(crate) fn span_for(index: usize, len: usize) -> Span {
    Span::new(index as u32, (index + len) as u32)
}

pub(crate) fn find_all<'a>(
    source_text: &'a str,
    pattern: &'a str,
) -> impl Iterator<Item = usize> + 'a {
    source_text.match_indices(pattern).map(|(index, _)| index)
}

pub(crate) fn line_prefix(source_text: &str, index: usize) -> &str {
    let line_start = source_text[..index].rfind('\n').map_or(0, |line| line + 1);
    &source_text[line_start..index]
}

pub(crate) fn quoted_value_after(source_text: &str, start: usize) -> Option<(&str, usize, usize)> {
    let quote = source_text.as_bytes().get(start).copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let value_start = start + 1;
    let value_end = source_text[value_start..]
        .find(quote as char)
        .map(|offset| value_start + offset)?;
    Some((&source_text[value_start..value_end], value_start, value_end))
}

pub(crate) fn count_occurrences(source_text: &str, pattern: &str) -> usize {
    source_text.match_indices(pattern).count()
}
