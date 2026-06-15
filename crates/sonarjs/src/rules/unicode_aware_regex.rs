//! Rule `unicode-aware-regex` (SonarJS key S5867).
//!
//! A Unicode property escape `\p{...}` or `\P{...}` in a regular expression
//! only works correctly when the `u` (unicode) or `v` (unicodeSets) flag is
//! present. Without one of those flags the engine treats `\p` as a literal `p`,
//! so the pattern silently matches the wrong things instead of selecting
//! characters by their Unicode property.
//!
//! **Flagged**: `/\p{Letter}/` (no flags), `/\P{ASCII}/i` (has `i` but not
//! `u`/`v`).
//!
//! **Not flagged**: `/\p{Letter}/u`, `/\P{ASCII}/v`, `/abc/` (no property
//! escape), `/\p/` (no `{` following `\p`).
//!
//! **Known under-report**: `\u{NNNN}` hex code-point escapes are a different
//! construct and are not checked here.
//!
//! Behaviour is reproduced from the public RSPEC description (S5867) only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{RegExpFlags, RegExpLiteral};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "unicode-aware-regex";

/// Returns `true` if the raw regex pattern text contains an unescaped `\p{`
/// or `\P{` sequence (a Unicode property escape).
///
/// Escaped backslashes (`\\`) are consumed as a pair so that `\\p{` (a literal
/// backslash followed by `p{`) is not misidentified as a Unicode property
/// escape.
fn has_unicode_property_escape(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i + 2 < len {
        if bytes[i] == b'\\' {
            if matches!(bytes[i + 1], b'p' | b'P') && bytes[i + 2] == b'{' {
                return true;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false
}

impl Scanner<'_> {
    pub(crate) fn check_unicode_aware_regex(&mut self, lit: &RegExpLiteral<'_>) {
        if lit.regex.flags.intersects(RegExpFlags::U | RegExpFlags::V) {
            return;
        }
        let pattern = lit.regex.pattern.text.as_str();
        if has_unicode_property_escape(pattern) {
            self.report(RULE_NAME, "unicodeAwareRegex", lit.span);
        }
    }
}
