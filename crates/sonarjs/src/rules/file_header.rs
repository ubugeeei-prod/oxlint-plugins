//! Rule `file-header` (SonarJS key S1451).
//!
//! Clean-room port. Some organisations require every source file to begin with
//! a fixed, predefined header (a copyright or licence banner, for example).
//! This rule verifies that the file starts with the configured header text and
//! reports a single file-level issue when it does not.
//!
//! The expected header is supplied entirely by configuration through the
//! `headerFormat` option (mirrored on the Rust side as
//! [`SonarjsOptions::file_header_format`]). When no header is configured the
//! option is the empty string and the rule is inactive — exactly matching
//! SonarJS, whose default `headerFormat` is empty. Because the rule does
//! nothing until a project opts in by configuring a header, it never produces
//! a false positive on an unconfigured codebase.
//!
//! ## Matching and the narrow form
//!
//! The literal comparison checks that the file *begins with* the configured
//! header. Line endings are normalised on both sides (`\r\n`, `\r`, and `\n`
//! are all treated as a single line break) so that a header authored with Unix
//! newlines still matches a file saved with Windows newlines, and vice versa.
//! No allocation is performed: both strings are walked in lock-step with
//! on-the-fly newline normalisation.
//!
//! SonarJS additionally offers an `isRegularExpression` mode in which
//! `headerFormat` is treated as a regular expression matched against the start
//! of the file. The Rust core has no general regular-expression matching
//! engine, so this port deliberately restricts itself to the literal-string
//! form: when `isRegularExpression` is set the check is skipped (it
//! under-reports rather than risk a false positive). Regex-mode support is a
//! documented follow-up.
//!
//! One diagnostic is emitted per file, anchored at the very start of the file
//! (line 1), which is where a missing or incorrect header must be fixed.
//!
//! Behaviour is reproduced from the public RSPEC description (S1451) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "file-header";

impl Scanner<'_> {
    pub(crate) fn check_file_header(&mut self) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let header = self.options.file_header_format.as_str();
        // An empty header means "no header configured": the rule is inactive.
        if header.is_empty() {
            return;
        }
        // Regular-expression headers are out of scope for this narrow port.
        if self.options.file_header_is_regular_expression {
            return;
        }
        if Self::source_starts_with_header(self.source_text, header) {
            return;
        }
        self.report(RULE_NAME, "fileHeader", Span::new(0, 0));
    }

    /// Returns `true` when `source` begins with `header`, treating any of
    /// `\r\n`, `\r`, or `\n` as an equivalent single line break on both sides.
    /// Both strings are consumed in lock-step without allocating.
    fn source_starts_with_header(source: &str, header: &str) -> bool {
        let mut source_chars = source.chars().peekable();
        let mut header_chars = header.chars().peekable();
        loop {
            let Some(expected) = next_normalized(&mut header_chars) else {
                // Whole header consumed and matched.
                return true;
            };
            match next_normalized(&mut source_chars) {
                Some(actual) if actual == expected => {}
                _ => return false,
            }
        }
    }
}

/// Pulls the next character from `chars`, collapsing a `\r` (optionally
/// followed by `\n`) into a single `\n` so that line-ending style does not
/// affect the comparison.
fn next_normalized(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<char> {
    match chars.next() {
        Some('\r') => {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            Some('\n')
        }
        other => other,
    }
}
