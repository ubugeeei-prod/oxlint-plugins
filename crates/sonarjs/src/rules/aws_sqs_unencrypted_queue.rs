//! Rule `aws-sqs-unencrypted-queue` (SonarJS key S6330).
//!
//! Clean-room port from public RSPEC S6330 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Amazon SQS can transparently encrypt the messages it stores at rest. An SQS
//! queue declared in AWS CDK code with encryption explicitly turned off leaves
//! its messages unencrypted, which is a risk when they carry sensitive data or
//! when a compliance regime mandates encryption (CWE-311).
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` when EITHER of two signals appears,
//! both of which are distinctive to SQS queue configuration:
//!
//! - key `encryption` whose value is a member expression whose terminal
//!   property is `UNENCRYPTED` (the `QueueEncryption.UNENCRYPTED` enum member
//!   used by the high-level `sqs.Queue` construct); or
//! - key `sqsManagedSseEnabled` whose value is the boolean literal `false`
//!   (the property used by the low-level `CfnQueue` construct).
//!
//! The property span is reported.
//!
//! ## Flagged
//! ```js
//! new Queue(this, 'q', { encryption: sqs.QueueEncryption.UNENCRYPTED });
//! new CfnQueue(this, 'q', { sqsManagedSseEnabled: false });
//! ```
//!
//! ## Not Flagged
//! ```js
//! new Queue(this, 'q', { encryption: QueueEncryption.KMS });          // encrypted
//! new Queue(this, 'q', { encryption: QueueEncryption.SQS_MANAGED });  // encrypted
//! new CfnQueue(this, 'q', { sqsManagedSseEnabled: true });            // encrypted
//! new Queue(this, 'q', { encryption: x });                           // non-literal
//! const c = { other: false };                                        // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-sqs-unencrypted-queue";

impl Scanner<'_> {
    pub(crate) fn check_aws_sqs_unencrypted_queue_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        let unencrypted = match key {
            "encryption" => matches!(
                &it.value,
                Expression::StaticMemberExpression(member)
                    if member.property.name.as_str() == "UNENCRYPTED"
            ),
            "sqsManagedSseEnabled" => {
                matches!(&it.value, Expression::BooleanLiteral(b) if !b.value)
            }
            _ => false,
        };
        if !unencrypted {
            return;
        }
        self.report(RULE_NAME, "sqsUnencrypted", it.span);
    }
}
