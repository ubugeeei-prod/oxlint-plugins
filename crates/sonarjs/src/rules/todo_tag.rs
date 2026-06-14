//! Rule `todo-tag` (SonarJS key S1135).
//!
//! Clean-room port. Flags comments containing the all-caps `TODO` tag, which
//! mark incomplete work that should be tracked and completed. Each comment that
//! contains the tag is reported once, at the comment's span.
//!
//! Scope/heuristic: the conventional all-caps `TODO` is matched as a
//! case-sensitive substring of the comment text (covers `// TODO`,
//! `/* TODO: ... */`, `// TODO do x`). Lowercase/mixed-case variants (`todo`,
//! `ToDo`) are intentionally not matched in this port; case-insensitive
//! matching would require allocation in the core and is a follow-up.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::Comment;
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "todo-tag";

impl Scanner<'_> {
    pub(crate) fn check_todo_tag(&mut self, comments: &[Comment]) {
        let mut spans: SmallVec<[Span; 8]> = SmallVec::new();
        for comment in comments {
            if self.text(comment.span).contains("TODO") {
                spans.push(comment.span);
            }
        }
        for span in spans {
            self.report(RULE_NAME, "todoTag", span);
        }
    }
}
