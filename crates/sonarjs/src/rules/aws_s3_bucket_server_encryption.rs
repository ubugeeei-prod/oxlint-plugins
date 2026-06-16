//! Rule `aws-s3-bucket-server-encryption` (SonarJS key S6245).
//!
//! Clean-room port from public RSPEC S6245 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Server-side encryption protects S3 bucket data at rest. Disabling it
//! exposes the stored objects to unauthorized access. In the AWS CDK the
//! bucket encryption mode is configured via the `encryption` `BucketProps`
//! property, and the explicit "no encryption" value is
//! `s3.BucketEncryption.UNENCRYPTED`. A managed mode (`S3_MANAGED`,
//! `KMS_MANAGED`, or `KMS`) should be used instead.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `encryption` and whose value (after
//! `get_inner_expression`) denotes the CDK UNENCRYPTED option, i.e. EITHER
//! (a) a static member expression whose terminal `property.name` is
//! `UNENCRYPTED` (matching `BucketEncryption.UNENCRYPTED` /
//! `s3.BucketEncryption.UNENCRYPTED`), OR (b) the string literal
//! `"UNENCRYPTED"`. The `encryption` key combined with the distinctive
//! `UNENCRYPTED` enum/string is the explicit "disable encryption" form, so
//! flagging only this pair is effectively zero-false-positive; no construct
//! gating is needed. The property span is reported.
//!
//! ## Flagged
//! ```js
//! new s3.Bucket(this, "b", { encryption: s3.BucketEncryption.UNENCRYPTED });
//! const x = { encryption: BucketEncryption.UNENCRYPTED };
//! const x = { encryption: "UNENCRYPTED" };
//! ```
//!
//! ## Not Flagged
//! ```js
//! { encryption: s3.BucketEncryption.KMS_MANAGED }   // managed encryption
//! { encryption: s3.BucketEncryption.S3_MANAGED }    // managed encryption
//! { encryption: someVariable }                      // non-literal value
//! { other: "UNENCRYPTED" }                          // different key
//! ```
//! Omission of `encryption` entirely is out of scope — only an explicit
//! `UNENCRYPTED` value is flagged.

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-s3-bucket-server-encryption";

impl Scanner<'_> {
    pub(crate) fn check_aws_s3_bucket_server_encryption_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "encryption" {
            return;
        }
        let is_unencrypted = match it.value.get_inner_expression() {
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "UNENCRYPTED"
            }
            Expression::StringLiteral(lit) => lit.value.as_str() == "UNENCRYPTED",
            _ => false,
        };
        if !is_unencrypted {
            return;
        }
        self.report(RULE_NAME, "s3BucketServerEncryption", it.span);
    }
}
