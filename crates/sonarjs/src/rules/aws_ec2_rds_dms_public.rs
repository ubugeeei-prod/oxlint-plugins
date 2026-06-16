//! Rule `aws-ec2-rds-dms-public` (SonarJS key S6329).
//!
//! Clean-room port from public RSPEC S6329 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Exposing an AWS cloud resource to the public Internet broadens its attack
//! surface: an EC2 instance, RDS database, or DMS replication instance that is
//! publicly reachable can be probed for data theft, intrusion, or disruption.
//! Such resources should stay on private subnets unless public access is a
//! deliberate, reviewed requirement.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `publiclyAccessible` or `associatePublicIpAddress`
//! and whose value is the boolean literal `true`. Both camelCase keys are
//! distinctive to AWS CDK networking configuration (RDS / DMS `publiclyAccessible`
//! and EC2 `associatePublicIpAddress`), so flagging only the `true` form is
//! effectively zero-false-positive in practice. The property span is reported.
//!
//! The subnet-type form (`vpcSubnets: { subnetType: PUBLIC }`) is intentionally
//! out of scope: it is far more prone to false positives and is not flagged.
//!
//! ## Flagged
//! ```js
//! new ec2.Instance(this, 'i', { publiclyAccessible: true });        // RDS/DMS/EC2 public
//! new ec2.CfnInstance(this, 'i', { networkInterfaces: [{ associatePublicIpAddress: true }] });
//! ```
//!
//! ## Not Flagged
//! ```js
//! const c = { publiclyAccessible: false };  // explicitly private
//! const c = { publiclyAccessible: x };       // non-literal value
//! const c = { other: true };                 // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-ec2-rds-dms-public";

impl Scanner<'_> {
    pub(crate) fn check_aws_ec2_rds_dms_public_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "publiclyAccessible" && key != "associatePublicIpAddress" {
            return;
        }
        let is_true = matches!(&it.value, Expression::BooleanLiteral(b) if b.value);
        if !is_true {
            return;
        }
        self.report(RULE_NAME, "ec2RdsDmsPublic", it.span);
    }
}
