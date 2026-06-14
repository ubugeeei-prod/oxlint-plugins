//! Rule `cyclomatic-complexity` (SonarJS key S1541).
//!
//! Clean-room port. Reports any function whose cyclomatic complexity exceeds the
//! configured threshold. Complexity starts at 1 (for the function itself) and
//! increments by 1 for each of the following decision-point nodes that appear
//! anywhere inside the function body:
//!
//! - `if` statement (every `if`, including those in `else if` chains — McCabe
//!   counts each branch test individually)
//! - `for` statement
//! - `for…in` statement
//! - `for…of` statement
//! - `while` statement
//! - `do…while` statement
//! - `case` clause in a `switch` statement (the `default` clause does NOT count)
//! - `catch` clause in a `try` statement
//! - conditional (ternary) expression (`?:`)
//! - logical expression (`&&`, `||`, `??`) — each LogicalExpression node adds +1
//!
//! Decision-point nodes at the top level (outside any function) do not count;
//! only nodes visited while a function frame is open contribute to complexity.
//! Nested functions each maintain their own independent frame on a stack, so
//! inner-function complexity does not inflate the outer function's count.
//!
//! A diagnostic fires when a function's complexity is **strictly greater than**
//! the configured `threshold` option (default **10**).
//!
//! Behaviour is reproduced from the public RSPEC S1541 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.

use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "cyclomatic-complexity";

impl Scanner<'_> {
    /// Pushes a new function frame onto the cyclomatic complexity stack with
    /// base complexity 1 (the function itself counts as one path). Called on
    /// entry to every function or arrow-function expression.
    pub(crate) fn enter_cyclomatic_scope(&mut self, span: Span) {
        self.cyclomatic_complexity_stack.push((span, 1));
    }

    /// Pops the top function frame and emits a diagnostic if the accumulated
    /// complexity exceeds the configured threshold. Called on exit from the
    /// function or arrow-function expression.
    pub(crate) fn leave_cyclomatic_scope(&mut self) {
        let Some((span, complexity)) = self.cyclomatic_complexity_stack.pop() else {
            return;
        };
        if complexity > self.options.cyclomatic_complexity_threshold {
            self.report(RULE_NAME, "cyclomaticComplexity", span);
        }
    }

    /// Increments the complexity counter of the innermost open function frame
    /// by 1. If no function frame is open (top-level code), this is a no-op so
    /// decision points outside any function are silently ignored.
    pub(crate) fn add_cyclomatic_complexity(&mut self) {
        if let Some(top) = self.cyclomatic_complexity_stack.last_mut() {
            top.1 += 1;
        }
    }
}
