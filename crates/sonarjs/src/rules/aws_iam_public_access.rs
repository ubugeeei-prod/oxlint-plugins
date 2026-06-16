//! Rule `aws-iam-public-access` (SonarJS key S6270).
//!
//! Clean-room port. AWS CDK IAM resource-based policies that grant access to a
//! wildcard principal expose the protected resource to every AWS account,
//! including anonymous and untrusted ones. In the CDK, that wildcard principal
//! is constructed via `new iam.AnyPrincipal()` (or an aliased
//! `new AnyPrincipal()`), the principal class whose semantics are "matches
//! everyone". Using it as a policy principal therefore grants public access.
//!
//! **Narrowing (zero-false-positive subset)**:
//! This port flags a `NewExpression` whose callee resolves to the distinctive
//! `AnyPrincipal` class name — either a bare identifier `AnyPrincipal` or a
//! static member expression ending in `.AnyPrincipal` (e.g. `iam.AnyPrincipal`).
//! The `AnyPrincipal` name is specific enough to the AWS IAM CDK API that this
//! cannot reasonably collide with unrelated code, keeping it false-positive
//! free. Any other principal class (`AccountRootPrincipal`, `ArnPrincipal`,
//! ...) is left alone, and a non-`new` reference to `AnyPrincipal` is not
//! flagged because only instantiation creates the public-access policy.
//!
//! The public RSPEC description (S6270) demonstrates only `AnyPrincipal`; no
//! `StarPrincipal` example is shown, so that name is intentionally not matched.
//!
//! Behaviour is reproduced from the public RSPEC description (S6270) only. No
//! upstream source, tests, fixtures, helper code, or message strings were
//! consulted or copied.
//!
//! ## Flagged
//! ```js
//! new iam.AnyPrincipal()
//! new AnyPrincipal()
//! ```
//!
//! ## Not flagged
//! ```js
//! new iam.AccountRootPrincipal()
//! new ArnPrincipal(arn)
//! AnyPrincipal // no `new`
//! ```

use oxc_ast::ast::{Expression, NewExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-iam-public-access";

impl<'a> Scanner<'a> {
    /// Reports a `NewExpression` whose callee is the AWS IAM `AnyPrincipal`
    /// class, which grants public access to all AWS accounts.
    pub(crate) fn check_aws_iam_public_access(&mut self, expr: &NewExpression<'a>) {
        let is_any_principal = match expr.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident.name.as_str() == "AnyPrincipal",
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "AnyPrincipal"
            }
            _ => false,
        };
        if is_any_principal {
            self.report(RULE_NAME, "iamPublicAccess", expr.span);
        }
    }
}
