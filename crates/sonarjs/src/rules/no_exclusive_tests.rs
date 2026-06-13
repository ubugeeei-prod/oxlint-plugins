//! Rule `no-exclusive-tests` (SonarJS key S6426).
//!
//! Clean-room port. Reports a `.only` member access on a known test-runner
//! function identifier. Calling `describe.only(...)`, `it.only(...)`, or
//! similar silently disables every other test in the suite — acceptable during
//! local debugging but disastrous if committed, because CI will only run the
//! focused test(s) and give a false green.
//!
//! **Covered forms**: `describe.only`, `it.only`, `test.only`, `context.only`,
//! `suite.only`, and `specify.only` where the object is a bare identifier
//! reference to one of those six known test-runner names. Chained or
//! namespaced forms (`myFramework.describe.only`) are not flagged because the
//! object after stripping parentheses is not a bare identifier.
//!
//! **Out of scope / follow-up**: Jasmine `fdescribe` / `fit` and other
//! framework-specific focused-test prefixes are not covered by this rule. They
//! can be added as a follow-up once the `.only` member-expression form is
//! stable in CI.
//!
//! Behaviour is reproduced from the public RSPEC description (S6426) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, StaticMemberExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-exclusive-tests";

const TEST_RUNNERS: [&str; 6] = ["describe", "it", "test", "context", "suite", "specify"];

impl Scanner<'_> {
    pub(crate) fn check_no_exclusive_tests(&mut self, member: &StaticMemberExpression<'_>) {
        if member.property.name.as_str() != "only" {
            return;
        }
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        if !TEST_RUNNERS.contains(&object.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "noExclusiveTests", member.span);
    }
}
