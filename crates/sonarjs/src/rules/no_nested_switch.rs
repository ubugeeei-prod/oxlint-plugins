//! Rule `no-nested-switch` (SonarJS key S1821).
//!
//! Clean-room port. Reports a `switch` statement that appears anywhere inside
//! another `switch` statement, because nested switches are hard to read.
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Semantics: a switch is flagged when at least one enclosing switch is open on
//! the traversal stack, so every nested switch is reported at its own `switch`
//! keyword regardless of nesting depth or any intervening statements (including
//! function bodies).

use oxc_ast::ast::SwitchStatement;
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-nested-switch";

/// Byte length of the `switch` keyword, used to report only the keyword token.
const SWITCH_KEYWORD_LEN: u32 = 6;

impl Scanner<'_> {
    pub(crate) fn check_no_nested_switch(&mut self, statement: &SwitchStatement<'_>) {
        if self.switch_depth > 0 {
            let start = statement.span.start;
            let keyword = Span::new(start, start + SWITCH_KEYWORD_LEN);
            self.report(RULE_NAME, "nestedSwitch", keyword);
        }
    }
}
