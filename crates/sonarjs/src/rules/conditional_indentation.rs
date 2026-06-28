//! Rule `conditional-indentation` (SonarJS key S3973).
//!
//! Clean-room port. When the body of an `if`, `for`, `for...in`, `for...of`,
//! `while`, or `do...while` statement is a single statement written WITHOUT
//! braces and placed on the line BELOW the control keyword, that statement
//! should be indented further than the keyword so the reader can see at a glance
//! which line is conditionally/iteratively executed. If the body sits at the
//! same indentation as (or less than) its control keyword, the layout is
//! misleading: the body reads like an independent statement that always runs.
//!
//! ```js
//! if (condition)
//! doSomething();        // Noncompliant: same column as `if`
//!
//! if (condition)
//!   doSomething();      // Compliant: indented past `if`
//! ```
//!
//! ## Narrow form
//!
//! To stay false-positive-free this port only flags the unambiguous case:
//!
//! - The body is NOT a `BlockStatement` (braces already make intent explicit).
//! - The body begins on a line strictly below the control keyword (a single-line
//!   `if (x) doSomething();` is never flagged).
//! - The body's start column is less than or equal to the keyword's start
//!   column.
//!
//! The comparison is by character column. If the leading text of either line
//! (keyword line or body line) contains a TAB, the check is skipped, because a
//! tab's visual width is editor-dependent and a raw column count could misjudge
//! the alignment. The `else` branch of an `if` is intentionally out of scope
//! (locating the `else` keyword's column reliably is left as a documented
//! follow-up); each `if` in an `else if` chain is still checked through its own
//! node, so chains are covered for their consequents.
//!
//! Behaviour is reproduced from the public RSPEC description (S3973) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{
    DoWhileStatement, ForInStatement, ForOfStatement, ForStatement, IfStatement, Statement,
    WhileStatement,
};
use oxc_span::{GetSpan, Span};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "conditional-indentation";

impl<'a> Scanner<'a> {
    pub(crate) fn check_conditional_indentation(&mut self, it: &IfStatement<'a>) {
        self.check_conditional_indentation_body(it.span, &it.consequent);
    }

    pub(crate) fn check_conditional_indentation_for(&mut self, it: &ForStatement<'a>) {
        self.check_conditional_indentation_body(it.span, &it.body);
    }

    pub(crate) fn check_conditional_indentation_for_in(&mut self, it: &ForInStatement<'a>) {
        self.check_conditional_indentation_body(it.span, &it.body);
    }

    pub(crate) fn check_conditional_indentation_for_of(&mut self, it: &ForOfStatement<'a>) {
        self.check_conditional_indentation_body(it.span, &it.body);
    }

    pub(crate) fn check_conditional_indentation_while(&mut self, it: &WhileStatement<'a>) {
        self.check_conditional_indentation_body(it.span, &it.body);
    }

    pub(crate) fn check_conditional_indentation_do_while(&mut self, it: &DoWhileStatement<'a>) {
        self.check_conditional_indentation_body(it.span, &it.body);
    }

    /// Shared core: `keyword_span` is the control statement's span (its start is
    /// the control keyword), `body` is the controlled statement.
    fn check_conditional_indentation_body(&mut self, keyword_span: Span, body: &Statement<'a>) {
        // A braced body makes the conditional region explicit; never flagged.
        if matches!(body, Statement::BlockStatement(_)) {
            return;
        }
        let body_span = body.span();
        let kw_loc = self.line_index.loc_for_span(self.source_text, keyword_span);
        let body_loc = self.line_index.loc_for_span(self.source_text, body_span);
        // Only the "body on the next line" layout is misleading; a single-line
        // body (or one wrapped above, which cannot happen here) is fine.
        if body_loc.start_line <= kw_loc.start_line {
            return;
        }
        // Tab in either line's leading text makes column counts unreliable.
        if self.indentation_has_tab(keyword_span.start) || self.indentation_has_tab(body_span.start)
        {
            return;
        }
        // Properly indented past the keyword: compliant.
        if body_loc.start_column > kw_loc.start_column {
            return;
        }
        self.report(RULE_NAME, "conditionalIndentation", body_span);
    }

    /// Returns `true` if the text from the start of `offset`'s line up to
    /// `offset` contains a tab character.
    fn indentation_has_tab(&self, offset: u32) -> bool {
        let offset = offset as usize;
        let line_start = self.source_text[..offset]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        self.source_text[line_start..offset]
            .bytes()
            .any(|b| b == b'\t')
    }
}
