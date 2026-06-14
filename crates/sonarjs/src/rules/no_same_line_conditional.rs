//! Rule `no-same-line-conditional` (SonarJS key S3972).
//!
//! Clean-room port. When an `if` statement begins on the same line as the
//! closing `}` of an immediately preceding sibling `if` statement, the intent is
//! ambiguous: the author may have meant `else if` (and forgotten the `else`), or
//! the two conditionals are independent and should be on separate lines. Either
//! way the layout invites a bug, so the second `if` is reported.
//!
//! ```js
//! if (a) {
//!   // ...
//! } if (b) {       // Noncompliant: `if` on the same line as the preceding `}`
//!   // ...
//! }
//! ```
//!
//! **Flagged**: a sibling `if` statement whose start line equals the end line of
//! the directly preceding sibling `if` statement (within the same statement
//! list: a program, block, function body, or switch case).
//!
//! **Not flagged**:
//! - `} else if (b) {` — the `else if` is part of one `if` statement, not a
//!   separate sibling.
//! - a second `if` placed on its own line.
//! - an `if` preceded by a non-`if` statement on the same line.
//!
//! Behaviour is reproduced from the public RSPEC description (S3972) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::Statement;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-same-line-conditional";

impl Scanner<'_> {
    pub(crate) fn check_no_same_line_conditional(&mut self, statements: &[Statement<'_>]) {
        let mut prev_if_end_line: Option<u32> = None;
        for statement in statements {
            let Statement::IfStatement(if_stmt) = statement else {
                prev_if_end_line = None;
                continue;
            };
            let loc = self.line_index.loc_for_span(self.source_text, if_stmt.span);
            if prev_if_end_line == Some(loc.start_line) {
                self.report(RULE_NAME, "sameLineConditional", if_stmt.span);
            }
            prev_if_end_line = Some(loc.end_line);
        }
    }
}
