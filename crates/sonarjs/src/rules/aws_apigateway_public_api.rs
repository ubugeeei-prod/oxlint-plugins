//! Rule `aws-apigateway-public-api` (SonarJS key S6333).
//!
//! Clean-room port from public RSPEC S6333 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Creating an AWS API Gateway method with no authorization unnecessarily
//! increases the attack surface: anyone can invoke the API without proving
//! their identity. In the AWS CDK this is expressed by setting the method's
//! `authorizationType` to `NONE` — either via the enum
//! `apigateway.AuthorizationType.NONE` (API Gateway v1) or the string literal
//! `"NONE"` (the `CfnRoute` / v2 escape hatch). Authentication should instead
//! use `AWS_IAM`, `COGNITO_USER_POOLS`, or a `CUSTOM` Lambda authorizer.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `authorizationType` and whose value is EITHER
//! (a) a static member expression whose terminal `property.name` is `NONE`
//! (matching `AuthorizationType.NONE` / `apigateway.AuthorizationType.NONE`),
//! OR (b) the string literal `"NONE"`. The `authorizationType` key combined
//! with an explicit `NONE` value is highly distinctive to API Gateway method
//! configuration, so flagging only this pair is effectively zero-false-positive.
//! The property span is reported.
//!
//! ## Flagged
//! ```js
//! resource.addMethod("GET", integration, {
//!   authorizationType: apigateway.AuthorizationType.NONE, // (a) enum NONE
//! });
//! new apigateway.CfnRoute(this, "no-auth", {
//!   authorizationType: "NONE",                            // (b) string NONE
//! });
//! ```
//!
//! ## Not Flagged
//! ```js
//! { authorizationType: apigateway.AuthorizationType.IAM }   // authenticated
//! { authorizationType: "AWS_IAM" }                          // authenticated
//! { authorizationType: someVariable }                       // non-literal value
//! { other: "NONE" }                                         // different key
//! ```
//! Omission of `authorizationType` entirely is out of scope — only an explicit
//! `NONE` value is flagged.

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-apigateway-public-api";

impl Scanner<'_> {
    pub(crate) fn check_aws_apigateway_public_api_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "authorizationType" {
            return;
        }
        let is_none = match &it.value {
            Expression::StaticMemberExpression(member) => member.property.name.as_str() == "NONE",
            Expression::StringLiteral(lit) => lit.value.as_str() == "NONE",
            _ => false,
        };
        if !is_none {
            return;
        }
        self.report(RULE_NAME, "apigatewayPublicApi", it.span);
    }
}
