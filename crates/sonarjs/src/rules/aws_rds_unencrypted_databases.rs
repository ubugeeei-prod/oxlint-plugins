//! Rule `aws-rds-unencrypted-databases` (SonarJS key S6303).
//!
//! Clean-room port from public RSPEC S6303 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An AWS CDK RDS database or cluster created with `storageEncrypted: false`
//! disables encryption at rest, exposing the stored data, backups, replicas
//! and snapshots to unauthorized access. The property should be `true` so AWS
//! encrypts the data transparently. The rule applies to `CfnDBCluster`,
//! `CfnDBInstance`, `DatabaseCluster`, `DatabaseClusterFromSnapshot`,
//! `DatabaseInstance` and `DatabaseInstanceReadReplica`.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `storageEncrypted` and whose value is the
//! boolean literal `false`. The camelCase `storageEncrypted` key is
//! distinctive to AWS RDS construct props, so flagging only
//! `storageEncrypted: false` is effectively zero-false-positive in practice;
//! no construct gating is needed. The property span is reported.
//!
//! ## Flagged
//! ```js
//! new DatabaseInstance(this, 'db', { storageEncrypted: false });
//! const x = { storageEncrypted: false };
//! ```
//!
//! ## Not Flagged
//! ```js
//! const x = { storageEncrypted: true };   // explicitly encrypted
//! const x = { storageEncrypted: flag };   // non-literal value
//! const x = { encrypted: false };         // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-rds-unencrypted-databases";

impl Scanner<'_> {
    pub(crate) fn check_aws_rds_unencrypted_databases_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "storageEncrypted" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "rdsUnencrypted", it.span);
    }
}
