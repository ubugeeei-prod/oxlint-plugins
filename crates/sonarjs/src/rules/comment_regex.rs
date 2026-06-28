//! Rule `comment-regex` (SonarJS key S124).
//!
//! Clean-room port. This rule lets a project flag every comment whose text
//! matches a configured regular expression. It is the generic "track comments
//! matching a pattern" check: teams use it to forbid ad-hoc markers, leftover
//! scaffolding notes, ticket references that should not ship, and so on.
//!
//! ## Behaviour
//!
//! The pattern is supplied through the `regularExpression` option (mapped into
//! the Rust core as [`crate::SonarjsOptions::comment_regex_format`]). The
//! pattern is compiled once per file with the Rust `regex` engine and tested,
//! unanchored, against the full source text of each comment (delimiters
//! included). Every comment that matches yields one diagnostic at the comment's
//! span.
//!
//! When the configured pattern is the empty string the rule is a no-op — this
//! mirrors SonarJS, where the check does nothing until a regular expression is
//! configured. In the production adapter (`npm/sonarjs/index.js`) the option is
//! always forwarded, defaulting to `""` when the user has not set one, so an
//! enabled-but-unconfigured rule reports nothing.
//!
//! ## Narrow default
//!
//! There is no upstream default regular expression for S124. So that the rule
//! has meaningful, false-positive-free behaviour out of the box (and so the
//! Rust test harness, which cannot inject per-test options, can exercise it),
//! the core default pattern is the literal `XXX` — a conventional placeholder
//! marker comparable to the `TODO`/`FIXME` markers handled by sibling rules.
//! Any configured `regularExpression` overrides this default entirely.
//!
//! Regex `flags` (the third SonarJS option) are not honoured separately; callers
//! who need case-insensitivity or other modes should embed an inline flag group
//! such as `(?i)` in the pattern. An invalid pattern is treated as "no match"
//! rather than raising an error, so a malformed configuration never crashes the
//! scan.
//!
//! Behaviour is reproduced from the public RSPEC description (S124) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::Comment;
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;
use regex::Regex;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "comment-regex";

impl Scanner<'_> {
    pub(crate) fn check_comment_regex(&mut self, comments: &[Comment]) {
        let pattern = self.options.comment_regex_format.as_str();
        if pattern.is_empty() {
            return;
        }
        let Ok(regex) = Regex::new(pattern) else {
            return;
        };
        let mut spans: SmallVec<[Span; 8]> = SmallVec::new();
        for comment in comments {
            if regex.is_match(self.text(comment.span)) {
                spans.push(comment.span);
            }
        }
        for span in spans {
            self.report(RULE_NAME, "commentRegex", span);
        }
    }
}
