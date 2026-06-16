//! Rule `aws-s3-bucket-versioning` (SonarJS key S6252).
//!
//! Clean-room port from public RSPEC S6252 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! When versioning is disabled on an AWS S3 bucket, a new version of an object
//! silently overwrites the existing one and there is no way to recover the
//! previous content. Disabling versioning therefore exposes the bucket to
//! accidental or malicious data loss; versioning should be enabled for buckets
//! that hold information requiring high availability or recoverability.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `versioned` and whose value is the boolean
//! literal `false` — the explicit form used in AWS CDK S3 `BucketProps`
//! (e.g. `new s3.Bucket(this, 'id', { versioned: false })`). The `versioned`
//! key is distinctive to S3 bucket props, so flagging only `versioned: false`
//! is effectively zero-false-positive in practice. The property span is
//! reported.
//!
//! ## Deliberate under-report
//!
//! The upstream rule also flags the *absence* of the `versioned` property
//! (because the CDK default is unversioned). This port intentionally does NOT
//! flag omission: detecting it reliably would require knowing the surrounding
//! object is a CDK `BucketProps`, which is FP-prone across CDK versions and
//! unrelated APIs. Only the explicit `versioned: false` form is reported.
//!
//! ## Flagged
//! ```js
//! new s3.Bucket(this, 'b', { versioned: false }); // explicit disable
//! const props = { versioned: false };             // explicit disable
//! ```
//!
//! ## Not Flagged
//! ```js
//! new s3.Bucket(this, 'b', { versioned: true }); // explicitly secure
//! const props = { versioned: flag };             // non-literal value
//! new s3.Bucket(this, 'b', {});                  // omission (out of scope)
//! const props = { other: false };               // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-s3-bucket-versioning";

impl Scanner<'_> {
    pub(crate) fn check_aws_s3_bucket_versioning_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "versioned" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "s3BucketVersioning", it.span);
    }
}
