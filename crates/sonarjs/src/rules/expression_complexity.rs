//! Rule `expression-complexity` (SonarJS key S1067).
//!
//! Clean-room port. Reports any top-level logical or conditional expression whose
//! total operator count exceeds the configured threshold. "Top-level" means the
//! expression is not itself nested inside another counted operator; reporting
//! fires once at the outermost node of the over-complex chain.
//!
//! **Operators counted**: every `LogicalExpression` node (`&&`, `||`, `??`) and
//! every `ConditionalExpression` node (`?:`).
//!
//! **Aggregation boundary**: the count resets at every function or arrow-function
//! boundary so that operators inside a nested function body are scored
//! independently from the operators in the surrounding expression. A separate
//! base context covers module-level (top-level) expressions.
//!
//! A diagnostic fires when the accumulated operator count is **strictly greater
//! than** the configured `threshold` option (default **3**), i.e. 4 or more
//! operators in a single top-level expression are needed to trigger a report.
//!
//! **Examples**
//! - `a && b && c && d` → 3 operators, not flagged at default threshold 3.
//! - `a && b && c && d && e` → 4 operators, flagged (4 > 3).
//! - `a && b && c && d && e` inside `(x) => …` → counted in the arrow's own
//!   context, not the enclosing expression's context.
//!
//! Behaviour is reproduced from the public RSPEC S1067 description and observed
//! output only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.

use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "expression-complexity";

impl Scanner<'_> {
    /// Pushes a fresh expression-complexity context onto the stack.
    ///
    /// Called on entry to every function or arrow-function scope so that nested
    /// function bodies are scored independently from the enclosing expression.
    /// Also called at program entry to provide a base context for module-level
    /// (top-level) expressions.
    pub(crate) fn enter_expression_complexity_scope(&mut self) {
        self.expression_complexity_stack
            .push((0u32, 0u32, Span::new(0, 0)));
    }

    /// Pops the innermost expression-complexity context from the stack.
    ///
    /// Called on exit from a function or arrow-function scope, matching every
    /// `enter_expression_complexity_scope` call.
    pub(crate) fn leave_expression_complexity_scope(&mut self) {
        self.expression_complexity_stack.pop();
    }

    /// Records entry into a logical (`&&`, `||`, `??`) or conditional (`?:`)
    /// expression.
    ///
    /// Increments both the nesting depth and the running operator count in the
    /// current context. When the nesting depth rises from 0 to 1 the `span` of
    /// the current node is saved as the report location — that is the outermost
    /// operator of this chain and will be the diagnostic site.
    pub(crate) fn enter_expression_complexity_op(&mut self, span: Span) {
        let Some(top) = self.expression_complexity_stack.last_mut() else {
            return;
        };
        top.0 += 1;
        top.1 += 1;
        if top.0 == 1 {
            top.2 = span;
        }
    }

    /// Records exit from a logical or conditional expression.
    ///
    /// Decrements the nesting depth. When the depth returns to 0 — meaning we
    /// are leaving the outermost operator of the current chain — the accumulated
    /// count is compared against the configured threshold. If it exceeds the
    /// threshold a diagnostic is emitted at the saved outermost span, then the
    /// count and span are reset so the next independent expression starts fresh.
    pub(crate) fn leave_expression_complexity_op(&mut self) {
        let report_span = {
            let Some(top) = self.expression_complexity_stack.last_mut() else {
                return;
            };
            if top.0 == 0 {
                return;
            }
            top.0 -= 1;
            if top.0 == 0 {
                let should_report = top.1 > self.options.expression_complexity_threshold;
                let span = top.2;
                top.1 = 0;
                top.2 = Span::new(0, 0);
                should_report.then_some(span)
            } else {
                None
            }
        };
        if let Some(span) = report_span {
            self.report(RULE_NAME, "expressionComplexity", span);
        }
    }
}
