//! Rule `no-sonar-comments` (SonarJS key S1291).
//!
//! Clean-room port. Flags `NOSONAR` comments, which suppress SonarQube/SonarJS
//! analysis on a line and so can hide real issues; they should be removed and
//! the underlying problem fixed (or a documented, reviewed suppression used).
//! Each comment that contains the tag is reported once, at the comment's span.
//!
//! Scope/heuristic: the conventional all-caps `NOSONAR` is matched as a
//! case-sensitive substring of the comment text. Lowercase/mixed-case variants
//! are intentionally not matched in this port; case-insensitive matching would
//! require allocation in the core and is a follow-up.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::Comment;
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-sonar-comments";

impl Scanner<'_> {
    pub(crate) fn check_no_sonar_comments(&mut self, comments: &[Comment]) {
        let mut spans: SmallVec<[Span; 8]> = SmallVec::new();
        for comment in comments {
            if self.text(comment.span).contains("NOSONAR") {
                spans.push(comment.span);
            }
        }
        for span in spans {
            self.report(RULE_NAME, "noSonarComments", span);
        }
    }
}
