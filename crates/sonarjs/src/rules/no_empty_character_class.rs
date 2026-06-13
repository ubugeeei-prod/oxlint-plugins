//! Rule `no-empty-character-class` (SonarJS key S2639).
//!
//! Clean-room port. A character class `[]` in a regular expression is empty:
//! it can never match any character, which means the whole regex can never
//! match anything. This is almost always a programmer mistake — a missing
//! character range, a forgotten escape, or an accidental transposition.
//!
//! ## What is flagged
//!
//! Any **regex literal** that contains a syntactic empty character class `[]`
//! (two square brackets with nothing between them):
//!
//! ```js
//! /a[]b/   // flagged — empty class, can never match
//! /[]/     // flagged — the entire pattern is an empty class
//! ```
//!
//! ## What is NOT flagged
//!
//! - `[^]` — a negated empty class. In JavaScript this *does* match any
//!   single character (it is equivalent to `[\s\S]`), so it is NOT empty
//!   and is intentionally left alone.
//! - `[abc]` — a non-empty class.
//! - `/a\[\]b/` — escaped brackets; these are literal bracket characters
//!   in the input, not a class syntax pair.
//! - `[a[]` — a class whose content is `a[`; the first `]` closes the class
//!   and there is no empty class.
//!
//! ## Scope: regex literals only
//!
//! This rule detects empty character classes in **regex literals** (e.g.
//! `/a[]b/`) only. The `new RegExp("[]")` string-argument form is out of
//! scope for this PR; a correct implementation would require string-value
//! tracking and is left as a follow-up.
//!
//! ## Detection strategy
//!
//! The visitor hook `visit_reg_exp_literal` is overridden. The raw pattern
//! string is obtained via `lit.regex.pattern.text.as_str()`, which is the
//! `text` field of `oxc_ast::ast::RegExpPattern` — a `Str<'a>` that holds
//! the pattern exactly as it appears in source between the slashes, without
//! the flags.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::RegExpLiteral;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-empty-character-class";

/// Returns `true` when the regex pattern string contains a syntactic empty
/// character class `[]`. The algorithm is byte-level and handles:
/// - backslash escapes: `\x` skips the next byte entirely
/// - negated empty classes `[^]`: the `^` makes the class non-empty, skip
/// - nested `[` inside an open class: treated as a literal character
fn has_empty_character_class(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0usize;
    let mut in_class = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 2; // skip the escaped char
                continue;
            }
            b'[' if !in_class => {
                // empty class is `[]` exactly (NOT `[^]`, which matches any char)
                if bytes.get(i + 1) == Some(&b']') {
                    return true;
                }
                in_class = true;
            }
            b']' if in_class => {
                in_class = false;
            }
            _ => {}
        }
        i += 1;
    }
    false
}

impl Scanner<'_> {
    pub(crate) fn check_no_empty_character_class(&mut self, lit: &RegExpLiteral<'_>) {
        // `lit.regex.pattern.text` is `Str<'a>` — the raw pattern between
        // the slashes, without flags, as it appears in source.
        let pattern = lit.regex.pattern.text.as_str();
        if has_empty_character_class(pattern) {
            self.report(RULE_NAME, "emptyCharacterClass", lit.span);
        }
    }
}
