//! Rule `no-ignored-exceptions` (SonarJS key S2486).
//!
//! Clean-room port. A `catch` block that silently ignores the caught exception
//! is almost certainly a bug; at minimum the exception should be logged or
//! rethrown so that it is not swallowed without trace.
//!
//! ## What is flagged
//!
//! A `catch` clause whose body block is completely empty AND whose body span
//! contains no comment of any kind. An empty body with a comment is treated as
//! an intentional, documented silent-ignore and is therefore NOT flagged.
//!
//! ## Not flagged
//!
//! - A `catch` clause with at least one statement in the body.
//! - An empty `catch` body that contains a comment (e.g. `/* expected */`).
//!
//! ## Flagged
//!
//! ```js
//! try { foo(); } catch (e) {}          // empty, no comment
//! try { foo(); } catch {}              // optional binding, empty, no comment
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::CatchClause;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-ignored-exceptions";

impl Scanner<'_> {
    pub(crate) fn check_no_ignored_exceptions(&mut self, catch_clause: &CatchClause<'_>) {
        if !catch_clause.body.body.is_empty() {
            return;
        }
        let body_span = catch_clause.body.span;
        let has_comment = self
            .comment_spans
            .iter()
            .any(|cs| cs.start >= body_span.start && cs.end <= body_span.end);
        if has_comment {
            return;
        }
        self.report(RULE_NAME, "ignoredException", catch_clause.span);
    }
}
