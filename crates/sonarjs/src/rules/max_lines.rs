//! Rule `max-lines` (SonarJS key S104).
//!
//! Clean-room port. Reports files whose number of code lines exceeds the
//! configured threshold, because very long files are hard to read, understand,
//! and maintain, and are usually a sign that the file needs to be split.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## What counts as a "code line"
//!
//! A physical line counts as a code line when it contains at least one
//! character that is NOT whitespace AND NOT inside a comment span. Blank lines
//! (containing only whitespace) and lines whose every non-whitespace character
//! falls inside a `// …` or `/* … */` comment are excluded from the count.
//!
//! ## Threshold
//!
//! The threshold mirrors SonarJS's configurable `maximum` option
//! (`self.options.max_lines_threshold`); when no option is supplied the
//! SonarJS default of **1000** is used.
//!
//! A diagnostic is emitted when the code-line count is **strictly greater
//! than** the threshold; a file with exactly the threshold lines is accepted.

use oxc_ast::ast::Comment;
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "max-lines";

/// Returns `true` when byte offset `abs` falls inside `comment`'s span.
fn span_covers(comment: &Comment, abs: u32) -> bool {
    comment.span.start <= abs && abs < comment.span.end
}

/// Returns `true` when the slice `source[start..end]` contains at least one
/// character that is not whitespace and not covered by any comment span.
fn line_has_code(source: &str, start: usize, end: usize, comments: &[Comment]) -> bool {
    for (offset, ch) in source[start..end].char_indices() {
        if ch.is_whitespace() {
            continue;
        }
        let abs = (start + offset) as u32;
        if !comments.iter().any(|c| span_covers(c, abs)) {
            return true;
        }
    }
    false
}

/// Counts the number of code lines in `source`, excluding blank lines and
/// lines that are entirely inside comment spans.
fn count_code_lines(source: &str, comments: &[Comment]) -> u32 {
    let mut count = 0u32;
    let mut start = 0usize;
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            if line_has_code(source, start, i, comments) {
                count += 1;
            }
            start = i + 1;
        }
    }
    // Handle a final line with no trailing newline.
    if start < source.len() && line_has_code(source, start, source.len(), comments) {
        count += 1;
    }
    count
}

impl Scanner<'_> {
    pub(crate) fn check_max_lines(&mut self, comments: &[Comment]) {
        let code_lines = count_code_lines(self.source_text, comments);
        if code_lines <= self.options.max_lines_threshold {
            return;
        }
        self.report(RULE_NAME, "maxLines", Span::new(0, 0));
    }
}
