//! Rule `no-skipped-tests` (SonarJS key S1607).
//!
//! Clean-room port. Skipped tests accumulate silently: once skipped, they may
//! never be re-enabled, rotting away while the author forgets why they were
//! disabled. CI passes, but coverage quietly shrinks.
//!
//! **Covered forms**:
//! 1. `.skip` member access on a known test-runner identifier:
//!    `describe.skip(...)`, `it.skip(...)`, `test.skip(...)`,
//!    `context.skip(...)`, `suite.skip(...)`, `specify.skip(...)`.
//!    A `StaticMemberExpression` whose property is `skip` and whose object is a
//!    bare identifier in the runner set is flagged. Chained or namespaced forms
//!    (`myFramework.describe.skip`) are not flagged because the object is not a
//!    bare identifier.
//! 2. `x`-prefixed Jasmine-style calls: `xit(...)`, `xdescribe(...)`,
//!    `xtest(...)`, `xcontext(...)`, `xspecify(...)`. A `CallExpression` whose
//!    callee (after stripping parentheses) is a bare identifier in the x-set.
//!
//! Both forms share the `skippedTest` messageId. The member-expression span is
//! reported for form 1; the call-expression span for form 2.
//!
//! Behaviour is reproduced from the public RSPEC description (S1607) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression, StaticMemberExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-skipped-tests";

const TEST_RUNNERS: [&str; 6] = ["describe", "it", "test", "context", "suite", "specify"];

const X_RUNNERS: [&str; 5] = ["xit", "xdescribe", "xtest", "xcontext", "xspecify"];

impl Scanner<'_> {
    pub(crate) fn check_no_skipped_tests_member(&mut self, member: &StaticMemberExpression<'_>) {
        if member.property.name.as_str() != "skip" {
            return;
        }
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        if !TEST_RUNNERS.contains(&object.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "skippedTest", member.span);
    }

    pub(crate) fn check_no_skipped_tests_call(&mut self, call: &CallExpression<'_>) {
        let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
            return;
        };
        if !X_RUNNERS.contains(&callee.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "skippedTest", call.span);
    }
}
