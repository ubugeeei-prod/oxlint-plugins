//! Rule `function-inside-loop` (SonarJS key S1515).
//!
//! Clean-room port. Reports a function declaration, function expression, or
//! arrow function expression that is created directly inside a loop body
//! (`for`, `for-in`, `for-of`, `while`, `do-while`) relative to the **nearest
//! enclosing function**. A function nested inside another function that is
//! itself inside a loop is NOT flagged: the inner function boundary resets the
//! loop context (its closure captures the inner function's locals, not the
//! loop variable).
//!
//! Immediately invoked function expressions (IIFEs) are exempt: they run once,
//! in place, so they do not suffer the closure-over-loop-variable problem.
//!
//! No closure/capture analysis is performed — like upstream SonarJS, every
//! eligible function created in a loop is flagged regardless of whether it
//! references loop-modified state.
//!
//! Behaviour is reproduced from the public RSPEC S1515 description and observed
//! plugin behaviour only; no upstream source, tests, fixtures, or message
//! strings were consulted or copied.

use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "function-inside-loop";

impl Scanner<'_> {
    /// Enters any function-like node. If the current (enclosing) function scope
    /// is currently inside a loop and this function is not an IIFE, the function
    /// is flagged. A fresh `0` loop-depth frame is then pushed for this
    /// function's own body so loops nested inside it are measured afresh.
    pub(crate) fn enter_function_inside_loop(&mut self, span: Span) {
        let in_loop = self
            .loop_depth_in_function
            .last()
            .is_some_and(|depth| *depth > 0);
        if in_loop && !self.iife_function_starts.contains(&span.start) {
            self.report(RULE_NAME, "noFunctionInLoop", span);
        }
        self.loop_depth_in_function.push(0);
    }

    /// Leaves any function-like node. Pops this function's loop-depth frame.
    pub(crate) fn leave_function_inside_loop(&mut self) {
        self.loop_depth_in_function.pop();
    }

    /// Enters a loop. Increments the loop depth of the current function scope.
    pub(crate) fn enter_loop_depth(&mut self) {
        if let Some(top) = self.loop_depth_in_function.last_mut() {
            *top += 1;
        }
    }

    /// Leaves a loop. Decrements the loop depth of the current function scope.
    pub(crate) fn leave_loop_depth(&mut self) {
        if let Some(top) = self.loop_depth_in_function.last_mut() {
            *top -= 1;
        }
    }
}
