//! Rule `aws-s3-bucket-granted-access` (SonarJS key S6265).
//!
//! Clean-room port from public RSPEC S6265 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An AWS CDK S3 bucket that is created with an `accessControl` set to one of
//! the canned ACLs that grant access beyond the bucket owner (`PUBLIC_READ`,
//! `PUBLIC_READ_WRITE`, `AUTHENTICATED_READ`) exposes the bucket's objects to
//! anonymous or to any authenticated AWS user. A private access control should
//! be used instead.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `accessControl` and whose value is a static
//! member expression whose terminal property name is one of the granting
//! `BucketAccessControl` enum members (`PUBLIC_READ`, `PUBLIC_READ_WRITE`,
//! `AUTHENTICATED_READ`). The `accessControl` key combined with those specific
//! enum member names is distinctive to the CDK S3 bucket configuration, so
//! flagging only this shape is effectively zero-false-positive. The property
//! span is reported.
//!
//! ## Flagged
//! ```js
//! new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PUBLIC_READ_WRITE });
//! new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PUBLIC_READ });
//! new s3.Bucket(this, 'b', { accessControl: BucketAccessControl.AUTHENTICATED_READ });
//! ```
//!
//! ## Not Flagged
//! ```js
//! new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PRIVATE }); // private ACL
//! new s3.Bucket(this, 'b', { accessControl: x });                             // non-member value
//! new s3.Bucket(this, 'b', { other: BucketAccessControl.PUBLIC_READ });        // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-s3-bucket-granted-access";

/// `BucketAccessControl` enum members that grant access beyond the bucket owner.
const GRANTING_ACCESS_CONTROLS: [&str; 3] =
    ["PUBLIC_READ_WRITE", "PUBLIC_READ", "AUTHENTICATED_READ"];

impl Scanner<'_> {
    pub(crate) fn check_aws_s3_bucket_granted_access_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "accessControl" {
            return;
        }
        let Expression::StaticMemberExpression(member) = &it.value else {
            return;
        };
        if !GRANTING_ACCESS_CONTROLS.contains(&member.property.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "s3PublicAccess", it.span);
    }
}
