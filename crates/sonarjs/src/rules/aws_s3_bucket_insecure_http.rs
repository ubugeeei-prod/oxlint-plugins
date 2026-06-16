//! Rule `aws-s3-bucket-insecure-http` (SonarJS key S6249).
//!
//! Clean-room port from public RSPEC S6249 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An AWS CDK S3 bucket accepts both HTTP and HTTPS requests by default. HTTP
//! is cleartext, so authorizing it exposes the transferred data to
//! eavesdropping and tampering. Setting `enforceSSL: true` on the bucket props
//! denies any request that is not sent over HTTPS. Leaving `enforceSSL: false`
//! keeps the insecure cleartext HTTP access enabled.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `enforceSSL` and whose value is the boolean
//! literal `false`. The camelCase `enforceSSL` key is distinctive to AWS CDK
//! S3 `BucketProps`, so flagging only `enforceSSL: false` is effectively
//! zero-false-positive in practice; no construct gating is needed. The
//! property span is reported.
//!
//! ## Deliberate under-report
//!
//! Like the related RDS-encryption rule, this port intentionally does NOT flag
//! the absence/omission of the `enforceSSL` property. A bucket created without
//! `enforceSSL` still authorizes HTTP under S6249, but detecting omission would
//! require construct gating and reliable type information, which would
//! introduce false positives. Omission is therefore out of scope and this port
//! deliberately under-reports.
//!
//! ## Flagged
//! ```js
//! new Bucket(this, 'b', { enforceSSL: false });
//! const x = { enforceSSL: false };
//! ```
//!
//! ## Not Flagged
//! ```js
//! const x = { enforceSSL: true };   // HTTPS enforced
//! const x = { enforceSSL: flag };   // non-literal value
//! const x = { encryption: false };  // different key
//! const x = {};                     // omission (out of scope)
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-s3-bucket-insecure-http";

impl Scanner<'_> {
    pub(crate) fn check_aws_s3_bucket_insecure_http_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "enforceSSL" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "s3BucketInsecureHttp", it.span);
    }
}
