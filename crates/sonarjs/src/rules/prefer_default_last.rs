//! Rule `prefer-default-last` (SonarJS key S4524).
//!
//! Clean-room port. The `default` clause of a `switch` statement should appear
//! as the last clause, because placing it anywhere else makes the structure
//! harder to read and understand.
//!
//! ## Detection
//!
//! If the `switch` has no `default` clause the rule is silent. When a `default`
//! clause is found (`SwitchCase.test` is `None`) and its position is not the
//! last element of `switch.cases`, the `default` keyword span (7 bytes) is
//! reported.
//!
//! Behaviour is reproduced from the public RSPEC S4524 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged form
//!
//! ```js
//! switch (x) {
//!   default: break;   // default is not last — flagged
//!   case 1: break;
//! }
//! ```
//!
//! ## Allowed form
//!
//! ```js
//! switch (x) {
//!   case 1: break;
//!   default: break;   // default is last — OK
//! }
//! ```

use oxc_ast::ast::SwitchStatement;
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-default-last";

impl Scanner<'_> {
    pub(crate) fn check_prefer_default_last(&mut self, switch: &SwitchStatement<'_>) {
        let Some((pos, default_case)) = switch
            .cases
            .iter()
            .enumerate()
            .find(|(_, c)| c.test.is_none())
        else {
            return;
        };
        if pos + 1 == switch.cases.len() {
            return;
        }
        let start = default_case.span.start;
        self.report(RULE_NAME, "defaultLast", Span::new(start, start + 7));
    }
}
