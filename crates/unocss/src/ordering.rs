//! Class-token ordering rules and the `is_unocss_token` heuristic.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::types::TokenPart;

pub(crate) fn sorted_class_string(input: &str) -> Option<CompactString> {
    let leading_len = input.len() - input.trim_start().len();
    let trailing_len = input.len() - input.trim_end().len();
    let leading = &input[..leading_len];
    let trailing = &input[input.len() - trailing_len..];
    let body = input.trim();
    if body.is_empty() || body.contains("${") {
        return None;
    }

    let tokens: SmallVec<[&str; 16]> = body.split_whitespace().collect();
    if tokens.len() < 2 || tokens.iter().any(|token| !is_unocss_token(token)) {
        return None;
    }
    let sorted = sort_class_tokens(tokens.as_slice());
    let sorted_body = join_tokens(sorted.as_slice());
    if sorted_body == body {
        return None;
    }
    let mut output = CompactString::new("");
    output.push_str(leading);
    output.push_str(&sorted_body);
    output.push_str(trailing);
    Some(output)
}

pub(crate) fn sort_class_tokens<'a>(tokens: &[&'a str]) -> SmallVec<[&'a str; 16]> {
    let mut parts: SmallVec<[TokenPart<'a>; 16]> = tokens
        .iter()
        .enumerate()
        .map(|(index, text)| TokenPart { text, index })
        .collect();
    parts.sort_by(|left, right| {
        class_rank(left.text, left.index).cmp(&class_rank(right.text, right.index))
    });
    parts.into_iter().map(|part| part.text).collect()
}

pub(crate) fn join_tokens(tokens: &[&str]) -> CompactString {
    let mut output = CompactString::new("");
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output.push_str(token);
    }
    output
}

pub(crate) fn prefix_with_space(prefix: &str) -> CompactString {
    let mut output = CompactString::from(prefix);
    output.push(' ');
    output
}

fn class_rank(token: &str, index: usize) -> (u16, u16, usize) {
    let base = token.rsplit(':').next().unwrap_or(token);
    if base == "flex" {
        return (10, 0, index);
    }
    if base.starts_with("flex-") {
        return (11, 0, index);
    }
    if base == "grid" {
        return (12, 0, index);
    }
    if base == "block" || base == "inline" || base == "inline-block" || base == "hidden" {
        return (13, 0, index);
    }
    if base.starts_with("text-") {
        return (20, 0, index);
    }
    if let Some(axis) = spacing_axis_rank(base, 'm') {
        return (30, axis, index);
    }
    if let Some(axis) = spacing_axis_rank(base, 'p') {
        return (40, axis, index);
    }
    if let Some(axis) = position_axis_rank(base) {
        return (50, axis, index);
    }
    if base.starts_with("gap") {
        return (60, 0, index);
    }
    if base.starts_with("rounded") {
        return (70, 0, index);
    }
    if base.starts_with("border") {
        return (80, 0, index);
    }
    if base.starts_with("bg-") {
        return (90, 0, index);
    }
    if base.starts_with("h-") {
        return (100, 0, index);
    }
    if base.starts_with("w-") {
        return (101, 0, index);
    }
    (10_000, 0, index)
}

/// Rank a margin (`family = 'm'`) or padding (`family = 'p'`) utility by axis.
///
/// Only matches genuine utilities (`m`, `m-1`, `p4`, `mx1`, `ml-2`); a bare word
/// that merely starts with an axis letter (`my`, `prose`, `play`, `previous`) is
/// rejected so the heuristic does not flag ordinary class names as orderable.
fn spacing_axis_rank(base: &str, family: char) -> Option<u16> {
    let mut chars = base.strip_prefix(family)?.chars();
    let rank = match chars.next() {
        // `m`/`p` alone, or directly followed by a value (`m-1`, `p4`).
        None | Some('-' | '0'..='9') => return Some(0),
        Some('x') => 1,
        Some('y') => 2,
        Some('l') => 3,
        Some('r') => 4,
        Some('b') => 5,
        Some('t') => 6,
        _ => return None,
    };
    // A real axis utility continues with a value (`mx1`, `ml-2`); reject `my`/`pr`.
    match chars.next() {
        Some('-' | '0'..='9') => Some(rank),
        _ => None,
    }
}

fn position_axis_rank(base: &str) -> Option<u16> {
    if base.starts_with("left-") {
        return Some(3);
    }
    if base.starts_with("right-") {
        return Some(4);
    }
    if base.starts_with("bottom-") {
        return Some(5);
    }
    if base.starts_with("top-") {
        return Some(6);
    }
    None
}

pub(crate) fn is_unocss_token(token: &str) -> bool {
    let base = token.rsplit(':').next().unwrap_or(token);
    base == "flex"
        || base == "grid"
        || base == "block"
        || base == "inline"
        || base == "inline-block"
        || base == "hidden"
        || base.starts_with("flex-")
        || base.starts_with("text-")
        || spacing_axis_rank(base, 'm').is_some()
        || spacing_axis_rank(base, 'p').is_some()
        || position_axis_rank(base).is_some()
        || base.starts_with("gap")
        || base.starts_with("rounded")
        || base.starts_with("border")
        || base.starts_with("bg-")
        || base.starts_with("h-")
        || base.starts_with("w-")
}
