//! Rule `no-nested-functions` (SonarJS key S2004).
//!
//! Clean-room port. Reports any function (function declaration, function
//! expression, or arrow function expression) nested at a depth **strictly
//! greater than** the configured `threshold` option (default **4**).
//!
//! A function at module/script scope is at depth 1. A function defined inside
//! that is at depth 2, and so on. With the default threshold of 4, the first
//! depth that is flagged is 5.
//!
//! All three function-like node kinds count: function declarations, function
//! expressions, and arrow function expressions.
//!
//! Behaviour is reproduced from the public RSPEC S2004 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.

use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-nested-functions";

impl Scanner<'_> {
    /// Enters any function-like node. Increments the nesting depth and reports
    /// if the new depth exceeds the configured threshold.
    pub(crate) fn enter_nested_function(&mut self, span: Span) {
        self.function_nesting_depth += 1;
        if self.function_nesting_depth > self.options.no_nested_functions_threshold {
            self.report(RULE_NAME, "noNestedFunctions", span);
        }
    }

    /// Leaves any function-like node. Decrements the nesting depth.
    pub(crate) fn leave_nested_function(&mut self) {
        self.function_nesting_depth -= 1;
    }
}
