//! Shared helper for parsing regex literals with `oxc_regular_expression`.
//!
//! Centralises the `LiteralParser` call and the span-offset arithmetic so that
//! individual rules can simply call [`with_parsed_regex_literal`] with a closure
//! and collect results as owned (non-borrowing) data.

use oxc_allocator::Allocator;
use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::Pattern;
use oxc_regular_expression::{LiteralParser, Options as RegExpOptions};

/// Parses the pattern of a regex *literal* and invokes `f` with the parsed AST,
/// returning `f`'s result.  On a parse error, returns `R::default()` (so callers
/// that collect spans simply get an empty collection).  Span positions in the
/// returned pattern are already offset to the source file via [`RegExpOptions`].
pub(crate) fn with_parsed_regex_literal<R, F>(lit: &RegExpLiteral<'_>, source_text: &str, f: F) -> R
where
    R: Default,
    F: for<'alloc> FnOnce(&Pattern<'alloc>) -> R,
{
    let pattern_text = lit.regex.pattern.text.as_str();
    let start = lit.span.start as usize;
    let flags_start = start + 1 + pattern_text.len() + 1;
    let flags = &source_text[flags_start..lit.span.end as usize];
    let allocator = Allocator::default();
    let parsed = LiteralParser::new(
        &allocator,
        pattern_text,
        Some(flags),
        RegExpOptions {
            pattern_span_offset: lit.span.start + 1,
            flags_span_offset: flags_start as u32,
        },
    )
    .parse();
    match parsed {
        Ok(pattern) => f(&pattern),
        Err(_) => R::default(),
    }
}
