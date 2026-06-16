//! Rule `aws-s3-bucket-public-access` (SonarJS key S6281).
//!
//! Clean-room port from public RSPEC S6281 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Amazon S3 buckets are private by default, but an AWS CDK `BlockPublicAccess`
//! configuration can weaken that protection. Each of its four boolean settings
//! contributes to blocking public exposure; setting any of them to `false`
//! disables part of the protection and may expose the bucket's contents to the
//! public, which is rarely intended.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is one of the four `BlockPublicAccess` sub-keys —
//! `blockPublicAcls`, `blockPublicPolicy`, `ignorePublicAcls`, or
//! `restrictPublicBuckets` — and whose value is the boolean literal `false`.
//! These camelCase keys are distinctive to S3 `BlockPublicAccess`
//! configuration objects, so flagging only `key: false` is effectively
//! zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! new s3.BlockPublicAccess({ blockPublicAcls: false });        // disables ACL block
//! new s3.BlockPublicAccess({ restrictPublicBuckets: false });  // disables restriction
//! ```
//!
//! ## Not Flagged
//! ```js
//! new s3.BlockPublicAccess({ blockPublicAcls: true });  // explicitly secure
//! new s3.BlockPublicAccess({ blockPublicAcls: x });     // non-literal value
//! new s3.BlockPublicAccess({ other: false });           // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-s3-bucket-public-access";

impl Scanner<'_> {
    pub(crate) fn check_aws_s3_bucket_public_access_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if !matches!(
            key,
            "blockPublicAcls" | "blockPublicPolicy" | "ignorePublicAcls" | "restrictPublicBuckets"
        ) {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "s3BucketPublicAccess", it.span);
    }
}
