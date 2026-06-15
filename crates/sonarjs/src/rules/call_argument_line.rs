//! Rule `call-argument-line` (SonarJS key S1472).
//!
//! Clean-room port. A function call's opening parenthesis (and therefore its
//! argument list) should begin on the SAME line as the end of the callee.
//! Writing the `(...)` on the line BELOW the function name is confusing: with
//! automatic semicolon insertion it can read as a separate statement, and the
//! call easily looks unrelated to the preceding expression.
//!
//! ```js
//! foo
//! (arg);   // Noncompliant: the call's `(` starts on a new line
//! ```
//!
//! **Flagged**: the call's open parenthesis starts on a different line from the
//! end of the callee (or, for a generic call `foo<T>(x)`, the end of the type
//! arguments).
//!
//! **Not flagged**:
//! - `foo(arg);` — the `(` is on the callee's line.
//! - a call whose arguments are wrapped across lines but whose `(` is still on
//!   the callee's line:
//!   ```js
//!   foo(
//!     a,
//!     b
//!   );
//!   ```
//! - `obj.method(x);` — the callee is the whole member expression `obj.method`,
//!   which ends right before the `(` on the same line.
//!
//! The comparison is made on the OPEN PARENTHESIS line versus the callee-end
//! line, so multi-line argument lists are compliant as long as the `(` itself
//! does not move to a new line. The whole call expression is reported.
//!
//! `new Foo\n(x)` (a `NewExpression`) is intentionally out of scope here; only
//! `CallExpression` nodes are checked.
//!
//! Behaviour is reproduced from the public RSPEC description (S1472,
//! "Function call arguments should not start on new lines") only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::CallExpression;
use oxc_span::{GetSpan, Span};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "call-argument-line";

impl Scanner<'_> {
    pub(crate) fn check_call_argument_line(&mut self, call: &CallExpression<'_>) {
        // The token directly before the call's `(` is the end of the type
        // arguments for a generic call (`foo<T>(x)`), otherwise the callee. Its
        // end line is what the open paren must share.
        let prefix_span = match &call.type_arguments {
            Some(type_arguments) => type_arguments.span(),
            None => call.callee.span(),
        };
        let scan_start = prefix_span.end as usize;
        let scan_end = call.span.end as usize;
        let Some(paren_offset) = self.find_open_paren_offset(scan_start, scan_end) else {
            return;
        };
        let prefix_end_line = self
            .line_index
            .loc_for_span(self.source_text, prefix_span)
            .end_line;
        let paren_span = Span::new(paren_offset, paren_offset + 1);
        let paren_line = self
            .line_index
            .loc_for_span(self.source_text, paren_span)
            .start_line;
        if paren_line == prefix_end_line {
            return;
        }
        self.report(RULE_NAME, "sameLineAsCallee", call.span);
    }

    /// Returns the byte offset of the call's opening parenthesis, scanning
    /// forward from `scan_start` (the end of the callee or type arguments) and
    /// skipping whitespace, line comments, block comments, and optional-chaining
    /// punctuation. The first `(` outside a comment is the call's open paren, so
    /// the scan stops there.
    fn find_open_paren_offset(&self, scan_start: usize, scan_end: usize) -> Option<u32> {
        let bytes = self.source_text.as_bytes();
        let mut i = scan_start;
        while i < scan_end {
            match bytes[i] {
                b'(' => return Some(i as u32),
                b'/' if bytes.get(i + 1) == Some(&b'/') => {
                    i += 2;
                    while i < scan_end && bytes[i] != b'\n' {
                        i += 1;
                    }
                }
                b'/' if bytes.get(i + 1) == Some(&b'*') => {
                    i += 2;
                    while i < scan_end {
                        let closing = bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/');
                        i += 1;
                        if closing {
                            i += 1;
                            break;
                        }
                    }
                }
                _ => i += 1,
            }
        }
        None
    }
}
