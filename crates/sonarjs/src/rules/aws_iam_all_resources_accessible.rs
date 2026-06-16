//! Rule `aws-iam-all-resources-accessible` (SonarJS key S6304).
//!
//! Clean-room port from public RSPEC S6304 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An AWS IAM policy statement that grants access to every resource via the
//! wildcard `"*"` violates the principle of least privilege: it lets the
//! attached principal act on all resources in the account, which is rarely
//! intended and widens the blast radius of any credential compromise. Such
//! statements should list only the specific resources actually required.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `resources` and whose value is an array literal
//! that contains a string-literal element exactly equal to `"*"`. The
//! `resources: ["*"]` shape is distinctive to IAM `PolicyStatement`
//! configuration objects, so flagging only this exact shape is effectively
//! zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! new PolicyStatement({ actions: [a], resources: ["*"] }); // all resources
//! ```
//!
//! ## Not Flagged
//! ```js
//! new PolicyStatement({ resources: ["arn:aws:s3:::x"] }); // specific resource
//! new PolicyStatement({ resources: [] });                 // empty list
//! new PolicyStatement({ resources: x });                  // non-array value
//! new PolicyStatement({ other: ["*"] });                  // different key
//! ```

use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-iam-all-resources-accessible";

impl Scanner<'_> {
    pub(crate) fn check_aws_iam_all_resources_accessible_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "resources" {
            return;
        }
        let Expression::ArrayExpression(array) = &it.value else {
            return;
        };
        let grants_all = array.elements.iter().any(|element| {
            matches!(element, ArrayExpressionElement::StringLiteral(lit) if lit.value == "*")
        });
        if !grants_all {
            return;
        }
        self.report(RULE_NAME, "iamAllResources", it.span);
    }
}
