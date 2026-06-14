//! Faithful port of eslint-plugin-simple-import-sort's `shared.js`.
//!
//! Function names mirror the upstream JavaScript exactly so diffs are easy to
//! review side-by-side.

use oxc_ast::ast::{Comment, CommentKind, ImportDeclaration, ImportDeclarationSpecifier};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use unicode_normalization::UnicodeNormalization;

use crate::types::LineIndex;

// ---------------------------------------------------------------------------
// Constants (mirrors imports.js / exports.js)
// ---------------------------------------------------------------------------

/// Side-effect import style (style 0 in upstream)
pub(crate) const SIDE_EFFECT_STYLE: u8 = 0;

// ---------------------------------------------------------------------------
// Token types (mirrors shared.js token stream)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// `//` line comment – code includes the `//` prefix (no trailing newline)
    Line,
    /// `/* */` block comment – code includes `/*` and `*/`
    Block,
    /// Identifier, keyword, or specifier text
    Identifier,
    /// `,` punctuator
    Comma,
    /// Whitespace that contains no newline
    Spaces,
    /// `\n` or `\r\n` – a single newline token
    Newline,
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    /// The verbatim source text for this token
    pub(crate) code: CompactString,
}

impl Token {
    pub(crate) fn spaces(s: &str) -> Self {
        Token {
            kind: TokenKind::Spaces,
            code: CompactString::from(s),
        }
    }
    pub(crate) fn newline(s: &str) -> Self {
        Token {
            kind: TokenKind::Newline,
            code: CompactString::from(s),
        }
    }
    pub(crate) fn block(s: &str) -> Self {
        Token {
            kind: TokenKind::Block,
            code: CompactString::from(s),
        }
    }
    pub(crate) fn line(s: &str) -> Self {
        Token {
            kind: TokenKind::Line,
            code: CompactString::from(s),
        }
    }
    pub(crate) fn identifier(s: &str) -> Self {
        Token {
            kind: TokenKind::Identifier,
            code: CompactString::from(s),
        }
    }
    pub(crate) fn comma() -> Self {
        Token {
            kind: TokenKind::Comma,
            code: CompactString::from(","),
        }
    }
}

pub(crate) fn is_identifier(t: &Token) -> bool {
    t.kind == TokenKind::Identifier
}
pub(crate) fn is_block_comment(t: &Token) -> bool {
    t.kind == TokenKind::Block
}
pub(crate) fn is_line_comment(t: &Token) -> bool {
    t.kind == TokenKind::Line
}
pub(crate) fn is_spaces(t: &Token) -> bool {
    t.kind == TokenKind::Spaces
}
pub(crate) fn is_newline(t: &Token) -> bool {
    t.kind == TokenKind::Newline
}
pub(crate) fn has_newline(s: &str) -> bool {
    s.contains('\n')
}

// ---------------------------------------------------------------------------
// parseWhitespace (shared.js)
// ---------------------------------------------------------------------------

/// Split `whitespace` on `\r?\n`. If there are 5+ items (blank line), collapse
/// to first-spaces + first-newline + last-spaces. Emit `Spaces`/`Newline`
/// tokens, dropping empty Spaces tokens. Mirrors `parseWhitespace` in shared.js.
pub(crate) fn parse_whitespace(whitespace: &str) -> SmallVec<[Token; 4]> {
    let mut all_items: SmallVec<[&str; 8]> = SmallVec::new();
    let bytes = whitespace.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            all_items.push(&whitespace[start..i]);
            all_items.push(&whitespace[i..i + 2]);
            i += 2;
            start = i;
        } else if bytes[i] == b'\n' {
            all_items.push(&whitespace[start..i]);
            all_items.push(&whitespace[i..i + 1]);
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    all_items.push(&whitespace[start..]);

    // Collapse blank lines: if >=5 items, keep first 2 + last 1
    let items: SmallVec<[&str; 8]> = if all_items.len() >= 5 {
        let mut v: SmallVec<[&str; 8]> = SmallVec::new();
        v.extend_from_slice(&all_items[..2]);
        v.push(all_items[all_items.len() - 1]);
        v
    } else {
        all_items
    };

    let mut out: SmallVec<[Token; 4]> = SmallVec::new();
    for (index, s) in items.iter().enumerate() {
        if s.is_empty() {
            continue; // drop empty Spaces tokens
        }
        if index % 2 == 0 {
            out.push(Token::spaces(s));
        } else {
            out.push(Token::newline(s));
        }
    }
    out
}

/// Mirrors `removeBlankLines` = `printTokens(parseWhitespace(s))`.
pub(crate) fn remove_blank_lines(whitespace: &str) -> CompactString {
    print_tokens(&parse_whitespace(whitespace))
}

/// Collapse blank lines in the text of a single import/export statement.
///
/// Mirrors what upstream `getAllTokens` achieves: each gap *between* AST tokens
/// is run through `parseWhitespace`, collapsing blank lines (two or more
/// consecutive newlines → one). Crucially, string literals and comments are
/// individual tokens in upstream and are emitted verbatim, so whitespace *inside*
/// them must NOT be collapsed. We reproduce that with a tiny scanner that emits
/// string literals (`"…"`/`'…'`) and comments (`/* … */`, `// …`) untouched and
/// only collapses whitespace runs in the surrounding code. This is sound for
/// import/export statements, which contain no regex/division `/` ambiguity.
pub(crate) fn collapse_blank_lines_in_mixed_text(text: &str) -> CompactString {
    let mut out = CompactString::new("");
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    while i < len {
        let b = bytes[i];
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => {
                let start = i;
                while i < len && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                    i += 1;
                }
                out.push_str(&remove_blank_lines(&text[start..i]));
            }
            b'"' | b'\'' => {
                // String literal: copy verbatim through the matching unescaped quote.
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                out.push_str(&text[start..i.min(len)]);
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                // Block comment: copy verbatim through `*/`.
                let start = i;
                i += 2;
                while i < len && !(bytes[i] == b'*' && i + 1 < len && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                out.push_str(&text[start..i]);
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'/' => {
                // Line comment: copy verbatim up to (not including) the newline.
                let start = i;
                while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
                    i += 1;
                }
                out.push_str(&text[start..i]);
            }
            _ => {
                let ch = text[i..].chars().next().unwrap_or('\0');
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    out
}

/// Mirrors `printTokens`: join token.code.
pub(crate) fn print_tokens(tokens: &[Token]) -> CompactString {
    let mut out = CompactString::new("");
    for t in tokens {
        out.push_str(&t.code);
    }
    out
}

// ---------------------------------------------------------------------------
// Collator (shared.js `compare`)
// ---------------------------------------------------------------------------

/// Minimal general-category helper – only needs NonspacingMark detection.
#[derive(PartialEq, Eq)]
enum GeneralCategory {
    NonspacingMark,
    Other,
}

fn unicode_general_category(ch: char) -> GeneralCategory {
    let cp = ch as u32;
    // Unicode combining character ranges (category Mn)
    if (0x0300..=0x036F).contains(&cp)   // Combining Diacritical Marks
        || (0x1AB0..=0x1AFF).contains(&cp)  // Extended Combining
        || (0x1DC0..=0x1DFF).contains(&cp)  // Supplement
        || (0x20D0..=0x20FF).contains(&cp)  // For Symbols
        || (0xFE20..=0xFE2F).contains(&cp)
    // Half Marks
    {
        GeneralCategory::NonspacingMark
    } else {
        GeneralCategory::Other
    }
}

/// Base-level fold: lowercase + strip combining marks (NFD → drop Mn category).
/// Implements `sensitivity: "base"` from `Intl.Collator`.
fn base_fold(s: &str) -> CompactString {
    let mut out = CompactString::new("");
    for ch in s.nfd() {
        if unicode_general_category(ch) == GeneralCategory::NonspacingMark {
            continue;
        }
        for lc in ch.to_lowercase() {
            out.push(lc);
        }
    }
    out
}

/// ASCII collation rank table mirroring ICU DUCET order for the chars that
/// appear in our sort keys after `source_sort_key` transformation.
///
/// ICU root collation differs from codepoint order for several ASCII chars.
/// Empirically verified via `Intl.Collator("en", {sensitivity:"base", numeric:true})`:
///   NUL < _ < - < , < . < [ < ] < @ < / < # < ~  (all before digits, digits before letters)
///
/// Returns `None` for chars not in the table (fall back to codepoint order).
fn icu_ascii_rank(c: char) -> Option<u8> {
    match c {
        '\0' => Some(0),
        '_' => Some(1),
        '-' => Some(2),
        ',' => Some(3),
        '.' => Some(4),
        '[' => Some(5),
        ']' => Some(6),
        '@' => Some(7),
        '/' => Some(8),
        '#' => Some(9),
        '~' => Some(10),
        _ => None,
    }
}

/// Compare two pre-folded strings with numeric segments (numeric: true).
/// Uses an explicit ICU-ordered rank table for ASCII punctuation chars that
/// differ from codepoint order; falls back to codepoint for other chars.
fn collator_compare_base_numeric(a: &str, b: &str) -> i32 {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return 0,
            (None, Some(_)) => return -1,
            (Some(_), None) => return 1,
            (Some(ac), Some(bc)) => {
                let a_digit = ac.is_ascii_digit();
                let b_digit = bc.is_ascii_digit();
                if a_digit && b_digit {
                    let an = parse_num(&mut ai);
                    let bn = parse_num(&mut bi);
                    if an != bn {
                        return if an < bn { -1 } else { 1 };
                    }
                } else {
                    ai.next();
                    bi.next();
                    if ac != bc {
                        // Use ICU rank table for chars in our domain
                        let ra = icu_ascii_rank(ac);
                        let rb = icu_ascii_rank(bc);
                        let cmp = match (ra, rb) {
                            (Some(ra), Some(rb)) => ra.cmp(&rb),
                            (Some(_), None) => {
                                // table char vs non-table char: table chars are all
                                // punctuation/special → come before digits and letters
                                std::cmp::Ordering::Less
                            }
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => ac.cmp(&bc),
                        };
                        if cmp != std::cmp::Ordering::Equal {
                            return if cmp == std::cmp::Ordering::Less {
                                -1
                            } else {
                                1
                            };
                        }
                    }
                }
            }
        }
    }
}

fn parse_num<I: Iterator<Item = char>>(it: &mut std::iter::Peekable<I>) -> u64 {
    let mut n = 0u64;
    while let Some(&c) = it.peek() {
        if c.is_ascii_digit() {
            n = n.saturating_mul(10).saturating_add((c as u8 - b'0') as u64);
            it.next();
        } else {
            break;
        }
    }
    n
}

/// Mirrors `compare(a, b)` in shared.js:
/// `collator.compare(a, b) || (a < b ? -1 : a > b ? 1 : 0)`
pub(crate) fn compare(a: &str, b: &str) -> i32 {
    let fa = base_fold(a);
    let fb = base_fold(b);
    let cmp = collator_compare_base_numeric(&fa, &fb);
    if cmp != 0 {
        return cmp;
    }
    // Tiebreak: code-point order on original strings
    if a < b {
        -1
    } else if a > b {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Comment helpers
// ---------------------------------------------------------------------------

/// Text of a comment as it appears in source (including `//` or `/* */`).
pub(crate) fn comment_text<'a>(source_text: &'a str, comment: &Comment) -> &'a str {
    source_text
        .get(comment.span.start as usize..comment.span.end as usize)
        .unwrap_or("")
}

/// Mirrors `getCommentsBefore(node).filter(...)` in `getImportExportItems`.
///
/// Returns comments that are "before" `node_start` and satisfy the filter:
///   - `comment.start.line <= node_start_line`
///   - `comment.end.line > last_line`
///   - if first node (is_first_node): `comment.start.line > last_line`
pub(crate) fn comments_before_node<'a>(
    all_comments: &'a [Comment],
    source_text: &str,
    line_index: &LineIndex,
    node_start: u32,
    node_start_line: u32,
    last_line: u32,
    is_first_node: bool,
) -> SmallVec<[&'a Comment; 4]> {
    let mut result: SmallVec<[&'a Comment; 4]> = SmallVec::new();
    for comment in all_comments {
        if comment.span.start >= node_start {
            break;
        }
        // Comment must be wholly before the node
        if comment.span.end > node_start {
            continue;
        }
        let c_start_line = line_index.line_for_offset(source_text, comment.span.start);
        let c_end_line = line_index.line_for_offset(source_text, comment.span.end);
        if c_start_line <= node_start_line
            && c_end_line > last_line
            && (!is_first_node || c_start_line > last_line)
        {
            result.push(comment);
        }
    }
    result
}

/// Mirrors `getCommentsAfter(node).filter(c => c.loc.end.line === node.loc.end.line)`.
pub(crate) fn comments_after_node<'a>(
    all_comments: &'a [Comment],
    source_text: &str,
    line_index: &LineIndex,
    node_end: u32,
    node_end_line: u32,
    // If `Some(pos)`, only include comments that start BEFORE this position.
    // Used to exclude comments that belong to the next import/export node.
    next_node_start: Option<u32>,
) -> SmallVec<[&'a Comment; 4]> {
    let mut result: SmallVec<[&'a Comment; 4]> = SmallVec::new();
    for comment in all_comments {
        if comment.span.start < node_end {
            continue;
        }
        // Stop at the next node if provided
        if let Some(nns) = next_node_start
            && comment.span.start >= nns
        {
            break;
        }
        let c_end_line = line_index.line_for_offset(source_text, comment.span.end);
        if c_end_line == node_end_line {
            result.push(comment);
        } else {
            // Once we've passed the end line, no more can match
            break;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// printCommentsBefore / printCommentsAfter (shared.js)
// ---------------------------------------------------------------------------

/// Mirrors `printCommentsBefore(node, comments, sourceCode)`.
pub(crate) fn print_comments_before(
    source_text: &str,
    node_start: u32,
    comments: &[&Comment],
) -> CompactString {
    if comments.is_empty() {
        return CompactString::new("");
    }
    let last_index = comments.len() - 1;
    let mut out = CompactString::new("");
    for (index, comment) in comments.iter().enumerate() {
        let next_start = if index == last_index {
            node_start
        } else {
            comments[index + 1].span.start
        };
        out.push_str(comment_text(source_text, comment));
        let gap = source_text
            .get(comment.span.end as usize..next_start as usize)
            .unwrap_or("");
        out.push_str(&remove_blank_lines(gap));
    }
    out
}

/// Mirrors `printCommentsAfter(node, comments, sourceCode)`.
pub(crate) fn print_comments_after(
    source_text: &str,
    node_end: u32,
    comments: &[&Comment],
) -> CompactString {
    if comments.is_empty() {
        return CompactString::new("");
    }
    let mut out = CompactString::new("");
    for (index, comment) in comments.iter().enumerate() {
        let prev_end = if index == 0 {
            node_end
        } else {
            comments[index - 1].span.end
        };
        let gap = source_text
            .get(prev_end as usize..comment.span.start as usize)
            .unwrap_or("");
        out.push_str(&remove_blank_lines(gap));
        out.push_str(comment_text(source_text, comment));
    }
    out
}

// ---------------------------------------------------------------------------
// getIndentation / getTrailingSpaces (shared.js)
// ---------------------------------------------------------------------------

/// Split on `\r?\n` keeping delimiters (alternating: spaces, newline, spaces, ...)
pub(crate) fn split_on_newline(s: &str) -> SmallVec<[&str; 4]> {
    let mut result: SmallVec<[&str; 4]> = SmallVec::new();
    let bytes = s.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            result.push(&s[start..i]);
            result.push(&s[i..i + 2]);
            i += 2;
            start = i;
        } else if bytes[i] == b'\n' {
            result.push(&s[start..i]);
            result.push(&s[i..i + 1]);
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    result.push(&s[start..]);
    result
}

/// Mirrors `getIndentation(node, sourceCode)` where `tokenBefore` is known.
///
/// If `prev_end` is None (first node in file):
///   `lines = sourceCode.text[..node.start].split(NEWLINE)` → last element
/// If `prev_end` is Some:
///   `text = sourceCode.text[prev_end..node.start]`
///   `lines = text.split(NEWLINE)` → last element if len > 1, else ""
pub(crate) fn get_indentation(
    source_text: &str,
    prev_end: Option<u32>,
    node_start: u32,
) -> CompactString {
    match prev_end {
        None => {
            let text = source_text.get(..node_start as usize).unwrap_or("");
            let parts = split_on_newline(text);
            CompactString::from(*parts.last().unwrap_or(&""))
        }
        Some(prev) => {
            let text = source_text
                .get(prev as usize..node_start as usize)
                .unwrap_or("");
            let parts = split_on_newline(text);
            if parts.len() > 1 {
                CompactString::from(*parts.last().unwrap_or(&""))
            } else {
                CompactString::new("")
            }
        }
    }
}

/// Mirrors `getTrailingSpaces(node, sourceCode)`.
///
/// If `next_start` is None:
///   `text = sourceCode.text[last_end..]` → first element of split
/// If `next_start` is Some:
///   `text = sourceCode.text[last_end..next_start]` → first element of split
pub(crate) fn get_trailing_spaces(
    source_text: &str,
    last_end: u32,
    next_start: Option<u32>,
) -> CompactString {
    let text = match next_start {
        None => source_text.get(last_end as usize..).unwrap_or(""),
        Some(next) => source_text
            .get(last_end as usize..next as usize)
            .unwrap_or(""),
    };
    let parts = split_on_newline(text);
    CompactString::from(*parts.first().unwrap_or(&""))
}

// ---------------------------------------------------------------------------
// getSource / getImportExportKind (shared.js)
// ---------------------------------------------------------------------------

/// Mirrors `getSource(node)` – returns sort key components.
pub(crate) fn get_source(original: &str, kind_str: &str) -> SourceInfo {
    SourceInfo {
        source: source_sort_key(original),
        original_source: CompactString::from(original),
        kind: CompactString::from(kind_str),
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceInfo {
    pub(crate) source: CompactString,
    pub(crate) original_source: CompactString,
    pub(crate) kind: CompactString,
}

/// Mirrors the character substitutions in `getSource` / upstream `source` key.
pub(crate) fn source_sort_key(source: &str) -> CompactString {
    let mut s = CompactString::from(source);
    // ^[./]*\.$ → append /
    if s.chars().all(|c| c == '.' || c == '/') && s.ends_with('.') {
        s.push('/');
    }
    // ^[./]*\/$ → append ,
    if s.chars().all(|c| c == '.' || c == '/') && s.ends_with('/') {
        s.push(',');
    }
    let mut out = CompactString::new("");
    for ch in s.chars() {
        match ch {
            '.' => out.push('_'),
            '/' => out.push('-'),
            '_' => out.push('.'),
            '-' => out.push('/'),
            _ => out.push(ch),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Import style (imports.js `getStyle`)
// ---------------------------------------------------------------------------

/// Mirrors `getStyle` from imports.js.
/// Returns: 0=side-effect, 1=namespace, 2=default, 3=named-only
pub(crate) fn import_style(decl: &ImportDeclaration<'_>, source_text: &str) -> u8 {
    let Some(specifiers) = &decl.specifiers else {
        return SIDE_EFFECT_STYLE; // `import "A"`
    };
    if specifiers.is_empty() {
        // Check if there's a `{` in position: `import {} from "A"` or `import type {} from "A"`
        if has_open_brace_specifiers(source_text, decl) {
            return 3;
        }
        return SIDE_EFFECT_STYLE;
    }
    match &specifiers[0] {
        ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => 1,
        ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => 2,
        ImportDeclarationSpecifier::ImportSpecifier(_) => 3,
    }
}

fn has_open_brace_specifiers(source_text: &str, decl: &ImportDeclaration<'_>) -> bool {
    // Returns true if the import has a `{ ... }` specifier list (even if empty or
    // containing only comments). OXC gives empty `specifiers` when only comments
    // are present (e.g. `import { /* X */ } from "pkg"`), so we cannot rely solely
    // on `specifiers.is_empty()` – we check the source text for a `{` before `from`.
    let text = source_text
        .get(decl.span.start as usize..decl.span.end as usize)
        .unwrap_or("");
    // Find `{` before the `from` keyword
    let from_pos = text
        .find(" from ")
        .or_else(|| text.find("\tfrom "))
        .unwrap_or(text.len());
    text[..from_pos].contains('{')
}

// ---------------------------------------------------------------------------
// getSpecifierItems state machine (shared.js)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct SpecifierItem {
    pub(crate) before: SmallVec<[Token; 4]>,
    pub(crate) after: SmallVec<[Token; 4]>,
    pub(crate) specifier: SmallVec<[Token; 4]>,
    pub(crate) had_comma: bool,
}

pub(crate) struct SpecifierItemsResult {
    pub(crate) before: SmallVec<[Token; 4]>,
    pub(crate) after: SmallVec<[Token; 4]>,
    pub(crate) items: SmallVec<[SpecifierItem; 8]>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Before,
    Specifier,
    After,
}

fn make_empty_item() -> SpecifierItem {
    SpecifierItem {
        before: SmallVec::new(),
        after: SmallVec::new(),
        specifier: SmallVec::new(),
        had_comma: false,
    }
}

/// Mirrors `getSpecifierItems(tokens)` in shared.js exactly.
pub(crate) fn get_specifier_items(tokens: &[Token]) -> SpecifierItemsResult {
    let mut result = SpecifierItemsResult {
        before: SmallVec::new(),
        after: SmallVec::new(),
        items: SmallVec::new(),
    };

    let mut state = State::Before;
    let mut current = make_empty_item();

    for token in tokens {
        match state {
            State::Before => match &token.kind {
                TokenKind::Newline => {
                    current.before.push(token.clone());
                    if result.before.is_empty() && result.items.is_empty() {
                        result.before = current.before.clone();
                        current = make_empty_item();
                        state = State::Before;
                    }
                }
                TokenKind::Spaces | TokenKind::Block | TokenKind::Line => {
                    current.before.push(token.clone());
                }
                _ => {
                    // Identifier or Comma → specifier starts
                    if result.before.is_empty() && result.items.is_empty() {
                        result.before = current.before.clone();
                        current = make_empty_item();
                    }
                    state = State::Specifier;
                    current.specifier.push(token.clone());
                }
            },
            State::Specifier => match &token.kind {
                TokenKind::Comma => {
                    current.had_comma = true;
                    state = State::After;
                }
                _ => {
                    current.specifier.push(token.clone());
                }
            },
            State::After => match &token.kind {
                TokenKind::Newline => {
                    current.after.push(token.clone());
                    result.items.push(current);
                    current = make_empty_item();
                    state = State::Before;
                }
                TokenKind::Spaces | TokenKind::Line => {
                    current.after.push(token.clone());
                }
                TokenKind::Block => {
                    if has_newline(&token.code) {
                        result.items.push(current);
                        current = make_empty_item();
                        state = State::Before;
                        current.before.push(token.clone());
                    } else {
                        current.after.push(token.clone());
                    }
                }
                _ => {
                    // Another specifier
                    result.items.push(current);
                    current = make_empty_item();
                    state = State::Specifier;
                    current.specifier.push(token.clone());
                }
            },
        }
    }

    // Handle final state
    match state {
        State::Before => {
            // trailing whitespace after last comma
            result.after = current.before;
        }
        State::Specifier => {
            // No trailing comma – separate identifier from trailing whitespace
            let last_id_index = current.specifier.iter().rposition(is_identifier);

            let last_id =
                last_id_index.unwrap_or_else(|| current.specifier.len().saturating_sub(1));

            let sp_part: SmallVec<[Token; 4]> =
                current.specifier[..=last_id].iter().cloned().collect();
            let after_part: SmallVec<[Token; 4]> =
                current.specifier[last_id + 1..].iter().cloned().collect();

            // Find slice point in after_part
            let newline_idx_raw = after_part.iter().position(is_newline);
            let newline_idx = newline_idx_raw.map(|i| i + 1); // include the newline

            let multiline_block_idx = after_part
                .iter()
                .position(|t| is_block_comment(t) && has_newline(&t.code));

            let slice_index: Option<usize> = match (newline_idx, multiline_block_idx) {
                (Some(ni), Some(mi)) => Some(ni.min(mi)),
                (Some(ni), None) => Some(ni),
                (None, Some(mi)) => Some(mi),
                (None, None) => {
                    if ends_with_spaces(&after_part) {
                        Some(after_part.len() - 1)
                    } else {
                        None
                    }
                }
            };

            let (item_after, res_after) = match slice_index {
                None => (after_part, SmallVec::new()),
                Some(idx) => {
                    let a: SmallVec<[Token; 4]> = after_part[..idx].iter().cloned().collect();
                    let b: SmallVec<[Token; 4]> = after_part[idx..].iter().cloned().collect();
                    (a, b)
                }
            };

            current.specifier = sp_part;
            current.after = item_after;
            result.items.push(current);
            result.after = res_after;
        }
        State::After => {
            // Last item had trailing comma
            if ends_with_spaces(&current.after) {
                let last = current.after.pop().expect("just checked non-empty");
                result.after = {
                    let mut v: SmallVec<[Token; 4]> = SmallVec::new();
                    v.push(last);
                    v
                };
            }
            result.items.push(current);
        }
    }

    result
}

fn ends_with_spaces(tokens: &[Token]) -> bool {
    tokens.last().is_some_and(is_spaces)
}

// ---------------------------------------------------------------------------
// needsStartingNewline (shared.js)
// ---------------------------------------------------------------------------

/// Mirrors `needsStartingNewline(tokens)`.
pub(crate) fn needs_starting_newline(tokens: &[Token]) -> bool {
    let filtered: SmallVec<[&Token; 4]> = tokens.iter().filter(|t| !is_spaces(t)).collect();
    match filtered.first() {
        None => false,
        Some(first) => {
            is_line_comment(first) || (is_block_comment(first) && !has_newline(&first.code))
        }
    }
}

// ---------------------------------------------------------------------------
// guessNewline (shared.js)
// ---------------------------------------------------------------------------

pub(crate) fn guess_newline(source_text: &str) -> &'static str {
    // Mirror upstream `guessNewline` = `/(\r?\n)/.exec(text)`: the FIRST newline
    // in the file decides the style (not "any CRLF anywhere"), so a file whose
    // first line ends in LF is treated as LF even if a later line uses CRLF.
    match source_text.find('\n') {
        Some(index) if index > 0 && source_text.as_bytes()[index - 1] == b'\r' => "\r\n",
        Some(_) => "\n",
        None => "\n",
    }
}

// ---------------------------------------------------------------------------
// Tokenizing the { ... } interior (mirrors shared.js getAllTokens subset)
// ---------------------------------------------------------------------------

/// Build the token stream for the interior of `{ ... }` in an import/export.
///
/// Corresponds to `allTokens[openBraceIndex+1 .. closeBraceIndex]` in JS.
/// Each specifier's source text becomes an `Identifier` token; gaps between
/// them are scanned for commas, comments, and whitespace.
pub(crate) fn tokenize_specifier_interior(
    source_text: &str,
    open_brace_end: u32,
    close_brace_start: u32,
    specifier_spans: &[Span],
    all_comments: &[Comment],
) -> SmallVec<[Token; 16]> {
    let mut tokens: SmallVec<[Token; 16]> = SmallVec::new();

    if specifier_spans.is_empty() {
        let text = source_text
            .get(open_brace_end as usize..close_brace_start as usize)
            .unwrap_or("");
        for t in parse_whitespace(text) {
            tokens.push(t);
        }
        return tokens;
    }

    let mut cursor = open_brace_end;
    for span in specifier_spans {
        scan_gap_tokens(source_text, cursor, span.start, all_comments, &mut tokens);
        let spec_text = source_text
            .get(span.start as usize..span.end as usize)
            .unwrap_or("");
        // Collapse blank lines within the specifier text (e.g. `a\n\n  as\n\n  b`→`a\n  as\n  b`).
        // Upstream's getAllTokens tokenizes the specifier into individual keyword/identifier
        // tokens and calls parseWhitespace on each inter-token gap, achieving the same effect.
        let spec_text_collapsed = collapse_blank_lines_in_mixed_text(spec_text);
        tokens.push(Token::identifier(&spec_text_collapsed));
        cursor = span.end;
    }
    scan_gap_tokens(
        source_text,
        cursor,
        close_brace_start,
        all_comments,
        &mut tokens,
    );

    tokens
}

/// Scan text between `from` and `to`, emitting: comments (Block/Line) from
/// `all_comments` and whitespace/comma tokens from the raw text.
fn scan_gap_tokens(
    source_text: &str,
    from: u32,
    to: u32,
    all_comments: &[Comment],
    out: &mut SmallVec<[Token; 16]>,
) {
    if from >= to {
        return;
    }
    let gap_comments: SmallVec<[&Comment; 4]> = all_comments
        .iter()
        .filter(|c| c.span.start >= from && c.span.end <= to)
        .collect();

    let mut cursor = from;
    for comment in &gap_comments {
        scan_gap_text(source_text, cursor, comment.span.start, out);
        let ctext = source_text
            .get(comment.span.start as usize..comment.span.end as usize)
            .unwrap_or("");
        match comment.kind {
            CommentKind::Line => out.push(Token::line(ctext)),
            CommentKind::SingleLineBlock | CommentKind::MultiLineBlock => {
                out.push(Token::block(ctext))
            }
        }
        cursor = comment.span.end;
    }
    scan_gap_text(source_text, cursor, to, out);
}

/// Scan raw text (no comments) between `from` and `to`, emitting commas and
/// whitespace tokens. Whitespace is split via `parse_whitespace`.
fn scan_gap_text(source_text: &str, from: u32, to: u32, out: &mut SmallVec<[Token; 16]>) {
    if from >= to {
        return;
    }
    let text = source_text.get(from as usize..to as usize).unwrap_or("");
    let mut pending = CompactString::new("");
    for ch in text.chars() {
        if ch == ',' {
            if !pending.is_empty() {
                for t in parse_whitespace(&pending) {
                    out.push(t);
                }
                pending.clear();
            }
            out.push(Token::comma());
        } else {
            pending.push(ch);
        }
    }
    if !pending.is_empty() {
        for t in parse_whitespace(&pending) {
            out.push(t);
        }
    }
}

// ---------------------------------------------------------------------------
// printWithSortedSpecifiers (shared.js)
// ---------------------------------------------------------------------------

/// Mirrors `printWithSortedSpecifiers` in shared.js.
///
/// Re-orders specifiers inside `{ ... }` while preserving all whitespace and
/// comments. Falls back to original source if there are ≤1 specifiers or
/// no braces.
pub(crate) fn print_with_sorted_specifiers(
    source_text: &str,
    node_start: u32,
    node_end: u32,
    all_comments: &[Comment],
    specifier_spans: &[Span],
    // (external_name, local_name, kind_rank) for each specifier
    sort_keys: &[(CompactString, CompactString, u8)],
    newline: &str,
) -> CompactString {
    let node_text = source_text
        .get(node_start as usize..node_end as usize)
        .unwrap_or("");

    // ≤1 specifiers: no reordering possible. Upstream returns
    // `printTokens(getAllTokens(node))`, i.e. the whole node re-emitted with
    // `parseWhitespace` applied to every inter-token gap — which is exactly
    // `collapse_blank_lines_in_mixed_text` over the node text (it preserves
    // string literals and comments verbatim and collapses blank lines elsewhere).
    // This also correctly handles `import "x" with { … }` import attributes and
    // empty `import { } from "x"` lists without mistaking a `with`-clause `{` or
    // a comment `{` for a specifier brace.
    if specifier_spans.len() <= 1 {
        return collapse_blank_lines_in_mixed_text(node_text);
    }

    // >1 specifiers: locate the specifier braces (comment/string-aware so a `{`
    // inside a comment, or a later `with { … }` brace, is never mistaken for the
    // specifier list) and reorder the interior.
    let Some((open_rel_in_node, close_rel_in_node)) =
        find_brace_positions(source_text, node_start, specifier_spans)
    else {
        return CompactString::from(node_text);
    };

    let open_brace_abs = node_start + open_rel_in_node as u32; // position of `{`
    let open_brace_end = open_brace_abs + 1; // position after `{`
    let close_brace_start = node_start + close_rel_in_node as u32; // position of `}`

    // Tokenize the interior
    let interior_tokens = tokenize_specifier_interior(
        source_text,
        open_brace_end,
        close_brace_start,
        specifier_spans,
        all_comments,
    );

    let items_result = get_specifier_items(&interior_tokens);

    if items_result.items.len() != specifier_spans.len() {
        // State machine desync – fall back
        return CompactString::from(node_text);
    }

    // Build sort keys with indices for stable sort
    let keyed: SmallVec<[(CompactString, CompactString, u8, usize); 8]> = sort_keys
        .iter()
        .enumerate()
        .map(|(i, (ext, loc, kr))| (ext.clone(), loc.clone(), *kr, i))
        .collect();

    let sorted_indices = sort_specifier_items_indices(&keyed);

    // Determine trailing comma. Upstream checks whether the token before `}` is
    // a comma; equivalently, the last specifier item ended in the `after` state,
    // which the state machine records as `had_comma` on that final item. This is
    // robust to line/block comments between the comma and `}`.
    let has_trailing_comma = items_result.items.last().is_some_and(|item| item.had_comma);

    let last_index = sorted_indices.len() - 1;
    let mut sorted_tokens: SmallVec<[Token; 32]> = SmallVec::new();

    for (new_pos, &orig_idx) in sorted_indices.iter().enumerate() {
        let item = &items_result.items[orig_idx];
        let prev_item = if new_pos == 0 {
            None
        } else {
            Some(&items_result.items[sorted_indices[new_pos - 1]])
        };

        // maybeNewline
        if let Some(prev) = prev_item
            && needs_starting_newline(&item.before)
            && !prev.after.last().is_some_and(is_newline)
        {
            sorted_tokens.push(Token::newline(newline));
        }

        sorted_tokens.extend(item.before.iter().cloned());
        sorted_tokens.extend(item.specifier.iter().cloned());

        if new_pos < last_index || has_trailing_comma {
            sorted_tokens.push(Token::comma());
            sorted_tokens.extend(item.after.iter().cloned());
        } else {
            // Last item, no trailing comma: trim leading blank tokens from .after
            // if the item previously had a comma
            let trimmed_after = trim_after_for_last(item);
            sorted_tokens.extend(trimmed_after.iter().cloned());
        }
    }

    // Final maybe-newline before `after` tokens
    let needs_final_nl = needs_starting_newline(&items_result.after)
        && !sorted_tokens.last().is_some_and(is_newline);
    if needs_final_nl {
        sorted_tokens.push(Token::newline(newline));
    }

    // Reconstruct the full node text, collapsing blank lines in prefix and suffix
    let prefix = collapse_blank_lines_in_mixed_text(&node_text[..open_rel_in_node + 1]);
    let suffix = collapse_blank_lines_in_mixed_text(&node_text[close_rel_in_node..]);

    let mut out = CompactString::new("");
    out.push_str(&prefix);
    out.push_str(&print_tokens(&items_result.before));
    out.push_str(&print_tokens(&sorted_tokens));
    out.push_str(&print_tokens(&items_result.after));
    out.push_str(&suffix);
    out
}

/// Find the positions of the specifier-list `{` and `}` within the node text.
///
/// Locate the specifier-list braces of an import/export with >1 specifiers.
///
/// Returns `(open_rel, close_rel)` as byte offsets from `node_start`. The open
/// brace is the first real `{` in the statement and the close brace is the first
/// real `}` at/after the last specifier — "real" meaning not inside a string
/// literal or comment, so a `{`/`}` inside a comment, or a later `with { … }`
/// import-attributes brace, is never mistaken for the specifier list.
fn find_brace_positions(
    source_text: &str,
    node_start: u32,
    specifier_spans: &[Span],
) -> Option<(usize, usize)> {
    let last_spec_end = specifier_spans[specifier_spans.len() - 1].end;
    let node_text = source_text.get(node_start as usize..)?;

    let open_rel = find_real_byte(node_text, 0, b'{')?;
    let close_from = (last_spec_end.saturating_sub(node_start)) as usize;
    let close_rel = find_real_byte(node_text, close_from, b'}')?;
    if close_rel <= open_rel {
        return None;
    }
    Some((open_rel, close_rel))
}

/// Index of the first `target` byte at/after `from` that is not inside a string
/// literal (`"…"`/`'…'`) or comment (`/*…*/`, `//…`). `target` must not be a
/// string/comment delimiter (`{`/`}` are the only callers).
fn find_real_byte(text: &str, from: usize, target: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = from;
    while i < len {
        let b = bytes[i];
        if b == target {
            return Some(i);
        }
        match b {
            b'"' | b'\'' => {
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    let end = bytes[i] == b;
                    i += 1;
                    if end {
                        break;
                    }
                }
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i += 2;
                while i < len && !(bytes[i] == b'*' && i + 1 < len && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'/' => {
                while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// Trim after tokens for the last specifier when it had a comma but now doesn't.
fn trim_after_for_last(item: &SpecifierItem) -> SmallVec<[Token; 4]> {
    if !item.had_comma {
        return item.after.clone();
    }
    let non_blank = item
        .after
        .iter()
        .position(|t| !is_newline(t) && !is_spaces(t));
    match non_blank {
        None => SmallVec::new(),
        Some(idx) => item.after[idx..].iter().cloned().collect(),
    }
}

// ---------------------------------------------------------------------------
// sortSpecifierItems (shared.js)
// ---------------------------------------------------------------------------

/// Returns the sorted permutation of indices for the specifier items.
/// Sort key: (external_name, local_name, kind_rank, index).
pub(crate) fn sort_specifier_items_indices(
    keyed: &[(CompactString, CompactString, u8, usize)],
) -> SmallVec<[usize; 8]> {
    let mut indices: SmallVec<[usize; 8]> = (0..keyed.len()).collect();
    indices.sort_by(|&ia, &ib| {
        let a = &keyed[ia];
        let b = &keyed[ib];
        let c = compare(&a.0, &b.0);
        if c != 0 {
            return if c < 0 {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let c = compare(&a.1, &b.1);
        if c != 0 {
            return if c < 0 {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        a.2.cmp(&b.2).then(a.3.cmp(&b.3))
    });
    indices
}
