//! Rule `max-switch-cases` (SonarJS key S1479).
//!
//! Clean-room port. Reports a `switch` statement whose number of `case` (and
//! `default`) clauses exceeds the threshold, because a switch with too many
//! branches is hard to read and is usually better expressed as a lookup table
//! or through polymorphism.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Threshold
//!
//! The threshold is fixed at **30** (`MAX_CASES`). SonarJS exposes a
//! configurable `maximum` option, but this port has no per-rule options
//! infrastructure yet. Configurability is a follow-up task; for now the
//! default of 30 is hardcoded.
//!
//! ## Counting
//!
//! All entries in `SwitchStatement.cases` are counted, including both `case`
//! clauses and the optional `default` clause. A diagnostic is emitted when the
//! count is **strictly greater than** `MAX_CASES`.

use oxc_ast::ast::SwitchStatement;
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "max-switch-cases";

/// Maximum number of `case`/`default` clauses allowed in a single `switch`.
/// A switch with more than this many clauses is flagged.
const MAX_CASES: usize = 30;

impl Scanner<'_> {
    pub(crate) fn check_max_switch_cases(&mut self, switch: &SwitchStatement<'_>) {
        if switch.cases.len() <= MAX_CASES {
            return;
        }
        let start = switch.span.start;
        let keyword = Span::new(start, start + 6);
        self.report(RULE_NAME, "maxSwitchCases", keyword);
    }
}
