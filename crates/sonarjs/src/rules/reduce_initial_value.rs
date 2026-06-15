//! Rule `reduce-initial-value` (SonarJS key S6959).
//!
//! Clean-room port. `Array.prototype.reduce()` and `Array.prototype.reduceRight()`
//! accept an optional second argument: the initial accumulator value. When that
//! argument is omitted (i.e. the method is called with a single argument — just
//! the callback) two surprising things happen: the call throws a `TypeError` if
//! the array is empty, and otherwise the first (or last) element is silently used
//! as the initial accumulator and skipped by the callback. Supplying an explicit
//! initial value removes both hazards, so the rule asks for one.
//!
//! This port reports a `.reduce(cb)` / `.reduceRight(cb)` call that has **exactly
//! one argument** (no initial value), whose single argument is **not** a spread
//! element, and whose receiver is *provably* an array: either a direct
//! array-literal receiver (`[1, 2].reduce(fn)`) or an identifier that resolves
//! via semantic analysis to a never-reassigned binding whose initializer is an
//! array literal (`const a = [1, 2]; a.reduce(fn);`).
//!
//! The upstream rule is type-aware (it knows the receiver is an array from
//! TypeScript types). We have NO TypeScript type information, so when the
//! receiver's array-ness cannot be proven (function parameters, imports,
//! reassigned variables, member access, call results, etc.) the call is
//! conservatively skipped to avoid false positives. We deliberately under-report
//! relative to upstream rather than risk flagging a non-array `.reduce()` method;
//! this under-reporting is a known parity risk. A spread argument
//! (`arr.reduce(...args)`) is also conservatively skipped, since the real
//! argument count is not statically known.
//!
//! Reported at the call expression span.
//!
//! Behaviour is reproduced from the public SonarSource rule documentation only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `[1, 2, 3].reduce((a, b) => a + b);` — array literal, no initial value
//! - `const a = [1, 2]; a.reduce(fn);` — resolved array variable, one argument
//! - `[1, 2, 3].reduceRight(fn);` — same defect on the right-to-left variant
//!
//! ## Not flagged
//! - `[1, 2].reduce((a, b) => a + b, 0);` — an initial value is provided
//! - `obj.reduce(fn);` — receiver is not a provable array
//! - `arr.reduce(...args);` — spread argument; real argument count unknown
//! - `foo.bar();` — not a `reduce`/`reduceRight` call

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "reduce-initial-value";

impl<'a> Scanner<'a> {
    /// Reports a `.reduce()` / `.reduceRight()` call that omits the initial value
    /// (a single, non-spread argument) on a provable array receiver.
    pub(crate) fn check_reduce_initial_value(&mut self, call: &CallExpression<'a>) {
        if call.arguments.len() != 1 {
            return;
        }
        let Some(argument) = call.arguments.first() else {
            return;
        };
        if argument.is_spread() {
            return;
        }
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "reduce" && method != "reduceRight" {
            return;
        }
        if !self.reduce_receiver_is_array(member.object.get_inner_expression()) {
            return;
        }
        self.report(RULE_NAME, "provideInitialValue", call.span);
    }

    /// Conservatively decides whether `receiver` is a provable array: a direct
    /// array literal, or an identifier resolving to an array-literal initializer.
    fn reduce_receiver_is_array(&self, receiver: &Expression<'a>) -> bool {
        match receiver {
            Expression::ArrayExpression(_) => true,
            Expression::Identifier(ident) => match self.resolve_identifier_initializer(ident) {
                Some(init) => matches!(init.get_inner_expression(), Expression::ArrayExpression(_)),
                None => false,
            },
            _ => false,
        }
    }
}
