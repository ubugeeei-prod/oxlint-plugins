//! Rule `no-case-label-in-switch` (SonarJS key S1219).
//!
//! Clean-room port. A labeled statement that appears directly among the
//! consequent statements of a switch case is almost certainly a typo — the
//! programmer likely intended a `case` clause (e.g. wrote `foo:` when they
//! meant `case foo:`, or misspelled `default:`).
//!
//! ## Detection strategy
//!
//! Only **direct** children of a `SwitchCase.consequent` array are inspected.
//! If the labeled statement is nested inside a block (`{ foo: bar(); }`) that
//! itself lives inside a case, it is **not** flagged — it is a deliberate block
//! label and falls outside the documented scope of this rule. The `no-labels`
//! rule covers that usage if desired.
//!
//! Overlap with `no-labels` is intentional: both rules may fire for the same
//! node when both are enabled. `no-labels` is unconditional (any labeled
//! statement); this rule is focused exclusively on the misleading switch-case
//! context.
//!
//! The diagnostic is anchored on `label.label.span`, which covers only the
//! identifier part of the label before the colon, keeping the report concise
//! and pointing directly at the label name.
//!
//! Behaviour is reproduced from the public RSPEC S1219 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Statement, SwitchStatement};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-case-label-in-switch";

impl Scanner<'_> {
    pub(crate) fn check_no_case_label_in_switch(&mut self, switch: &SwitchStatement<'_>) {
        // Collect spans first (immutable borrow of switch param), then report.
        let mut spans: SmallVec<[Span; 4]> = SmallVec::new();
        for case in &switch.cases {
            for stmt in &case.consequent {
                if let Statement::LabeledStatement(label) = stmt {
                    spans.push(label.label.span);
                }
            }
        }
        for span in spans {
            self.report(RULE_NAME, "caseLabelInSwitch", span);
        }
    }
}
