//! Rule `comma-or-logical-or-case` (SonarJS key S3616).
//!
//! Clean-room port. Reports a `switch` `case` label whose test expression is a
//! logical-OR (`case a || b:`) or a comma/sequence expression (`case a, b:`),
//! because such a label evaluates to a single value rather than matching multiple
//! values as the author likely intended.
//!
//! Only the test expression's span is reported. The `default:` label (no test)
//! and `case a && b:` are intentionally left alone.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Expression, SwitchCase};
use oxc_span::GetSpan;
use oxc_syntax::operator::LogicalOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "comma-or-logical-or-case";

impl Scanner<'_> {
    pub(crate) fn check_comma_or_logical_or_case(&mut self, case: &SwitchCase<'_>) {
        let Some(test) = &case.test else {
            return;
        };
        let flagged = match test.get_inner_expression() {
            Expression::LogicalExpression(logical) => logical.operator == LogicalOperator::Or,
            Expression::SequenceExpression(_) => true,
            _ => false,
        };
        if !flagged {
            return;
        }
        self.report(RULE_NAME, "commaOrLogicalOrInCase", test.span());
    }
}
