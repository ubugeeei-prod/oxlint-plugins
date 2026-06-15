//! Rule `no-nested-incdec` (SonarJS key S881).
//!
//! Clean-room port. A `++` or `--` operator has a side effect (it mutates its
//! operand) *and* evaluates to a value, so hiding one inside a larger
//! expression makes the code hard to read: the reader has to remember both that
//! the variable changed and what the surrounding expression evaluated to.
//!
//! ## Narrow form
//!
//! This port reports the unambiguous case named directly by the rule: an
//! increment or decrement used as an argument of a function or constructor
//! call, where the mutation is easily missed.
//!
//! ```js
//! foo(i++);            // Noncompliant
//! arr.push(--count);   // Noncompliant
//! new Widget(n++);     // Noncompliant
//! ```
//!
//! **Not flagged**:
//! - `i++;` — a standalone update statement.
//! - `for (let i = 0; i < n; i++)` — the update clause of a `for` loop.
//!
//! Increments mixed with other operators in an arithmetic or index expression
//! (`a[i++]`, `x = i++ + 1`) are a documented follow-up. Behaviour is
//! reproduced from the public RSPEC description (S881) only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Argument, CallExpression, NewExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-nested-incdec";

impl Scanner<'_> {
    pub(crate) fn check_no_nested_incdec_call(&mut self, call: &CallExpression<'_>) {
        self.check_no_nested_incdec_arguments(&call.arguments);
    }

    pub(crate) fn check_no_nested_incdec_new(&mut self, new_expr: &NewExpression<'_>) {
        self.check_no_nested_incdec_arguments(&new_expr.arguments);
    }

    fn check_no_nested_incdec_arguments(&mut self, arguments: &[Argument<'_>]) {
        for argument in arguments {
            if let Argument::UpdateExpression(update) = argument {
                self.report(RULE_NAME, "nestedIncDec", update.span);
            }
        }
    }
}
