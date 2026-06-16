//! Rule `aws-iam-all-privileges` (SonarJS key S6302).
//!
//! Clean-room port from public RSPEC S6302 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An AWS IAM policy statement that grants every action via the wildcard
//! `"*"` violates the principle of least privilege: it hands the attached
//! principal unrestricted permissions, which is rarely intended and widens the
//! blast radius of any credential compromise. Such statements should list only
//! the specific actions actually required.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `actions` and whose value is an array literal
//! that contains a string-literal element exactly equal to `"*"`. The
//! `actions: ["*"]` shape is distinctive to IAM `PolicyStatement`
//! configuration objects, so flagging only this exact shape is effectively
//! zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! new PolicyStatement({ actions: ["*"], resources: [bucket] }); // grants all
//! ```
//!
//! ## Not Flagged
//! ```js
//! new PolicyStatement({ actions: ["s3:GetObject"] }); // specific actions
//! new PolicyStatement({ actions: [] });               // empty list
//! new PolicyStatement({ actions: x });                // non-array value
//! new PolicyStatement({ other: ["*"] });              // different key
//! ```

use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-iam-all-privileges";

impl Scanner<'_> {
    pub(crate) fn check_aws_iam_all_privileges_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "actions" {
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
        self.report(RULE_NAME, "iamAllPrivileges", it.span);
    }
}
