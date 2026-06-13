//! Rule `no-small-switch` (SonarJS key S1301).
//!
//! Clean-room port. A `switch` statement with fewer than two real `case`
//! clauses should be rewritten as an `if` statement because the switch
//! construct adds syntactic weight with no structural benefit.
//!
//! ## Counting
//!
//! Only `SwitchCase` entries whose `test` field is `Some(…)` are counted —
//! that is, real `case X:` clauses. The `default:` clause, whose `test` is
//! `None`, is deliberately excluded from the count. A `switch` with one `case`
//! and a `default` is just an if/else, and a `switch` with zero `case` clauses
//! (only `default`, or empty) is just a block.
//!
//! When the real-case count is **strictly less than 2** the `switch` keyword
//! span is reported.
//!
//! Behaviour is reproduced from the public RSPEC S1301 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::SwitchStatement;
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-small-switch";

impl Scanner<'_> {
    pub(crate) fn check_no_small_switch(&mut self, switch: &SwitchStatement<'_>) {
        let case_count = switch.cases.iter().filter(|c| c.test.is_some()).count();
        if case_count >= 2 {
            return;
        }
        let start = switch.span.start;
        self.report(RULE_NAME, "smallSwitch", Span::new(start, start + 6));
    }
}
