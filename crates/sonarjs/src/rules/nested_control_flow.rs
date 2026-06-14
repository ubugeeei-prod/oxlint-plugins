//! Rule `nested-control-flow` (SonarJS key S134).
//!
//! Clean-room port. Reports control-flow statements nested too deeply. The
//! nesting depth counter increments for each of the eight statement types that
//! are considered a level: `if`, `for`, `for…in`, `for…of`, `while`,
//! `do…while`, `switch`, and `try`. A diagnostic fires on the statement that
//! causes the depth to exceed the configured `maximumNestingLevel` option
//! (default **3**).
//!
//! **`else if` chains do not add a level.** An `if` statement that is the
//! `alternate` (the `else` branch) of a parent `if` is part of the same
//! conditional chain and is not counted as an additional nesting level. Deeper
//! statements inside an already-flagged one are NOT separately flagged.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{IfStatement, Statement};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "nested-control-flow";

impl Scanner<'_> {
    /// Enters a control-flow statement that counts toward nesting depth. Reports
    /// when the current depth already equals the threshold (i.e. this node would
    /// exceed it), then increments. Returns `true` (the caller must pair it with
    /// `leave_nested_control_flow(true)`).
    pub(crate) fn enter_nested_control_flow(&mut self, span: Span) -> bool {
        if self.control_flow_depth == self.options.nested_control_flow_threshold {
            self.report(RULE_NAME, "nestedControlFlow", span);
        }
        self.control_flow_depth += 1;
        true
    }

    /// Enters an `if` statement. An `else if` (an `if` that is the `alternate`
    /// of a parent `if`) does not count toward depth and is not checked; its
    /// own alternate-if is marked so the chain continues. Returns whether this
    /// node was counted (pass it to `leave_nested_control_flow`).
    pub(crate) fn enter_nested_control_flow_if(&mut self, it: &IfStatement<'_>) -> bool {
        if let Some(Statement::IfStatement(alternate)) = &it.alternate {
            self.else_if_starts.push(alternate.span.start);
        }
        if self.else_if_starts.contains(&it.span.start) {
            return false;
        }
        self.enter_nested_control_flow(it.span)
    }

    /// Leaves a control-flow statement, decrementing the depth only if it was
    /// counted.
    pub(crate) fn leave_nested_control_flow(&mut self, counted: bool) {
        if counted {
            self.control_flow_depth -= 1;
        }
    }
}
