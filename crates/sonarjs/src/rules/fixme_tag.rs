//! Rule `fixme-tag` (SonarJS key S1134).
//!
//! Clean-room port. A `FIXME` tag inside a comment marks code that is
//! known-broken and must be addressed before the code is shipped. Leaving
//! `FIXME` comments in production code is a signal that the software is in a
//! degraded state.
//!
//! This rule performs a case-SENSITIVE substring search for the exact token
//! `FIXME` inside every comment in the file. Mixed-case variants (`fixme`,
//! `FixMe`) are intentionally out of scope for this port — detecting them
//! would require allocating a lowercased copy of each comment, which is
//! avoided in the Rust core; case-insensitive matching is a follow-up.
//!
//! One diagnostic is emitted per comment that contains `FIXME`, at the
//! comment's span (which includes the `//` or `/* */` delimiters).
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::Comment;
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "fixme-tag";

impl Scanner<'_> {
    pub(crate) fn check_fixme_tag(&mut self, comments: &[Comment]) {
        let mut spans: SmallVec<[Span; 8]> = SmallVec::new();
        for comment in comments {
            if self.text(comment.span).contains("FIXME") {
                spans.push(comment.span);
            }
        }
        for span in spans {
            self.report(RULE_NAME, "fixmeTag", span);
        }
    }
}
