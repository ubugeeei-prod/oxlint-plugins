//! Rule `aws-sagemaker-unencrypted-notebook` (SonarJS key S6319).
//!
//! Clean-room port from the public RSPEC S6319 documentation only; no upstream
//! source, tests, fixtures, helper code, or message strings were consulted or
//! copied.
//!
//! "Using unencrypted SageMaker notebook instances is security-sensitive."
//! Amazon SageMaker notebook instances can encrypt the data stored on their
//! attached volume with a customer-managed KMS key. When no key is supplied the
//! notebook's data is left unencrypted at rest, exposing it should an adversary
//! gain access to the underlying storage. This is a security hotspot, not a
//! certain vulnerability — the absence of a key is the thing worth reviewing.
//!
//! In the AWS CDK this surfaces as the low-level `CfnNotebookInstance` construct
//! created without a `kmsKeyId` property. The public RSPEC shows exactly:
//!
//! ```js
//! new CfnNotebookInstance(this, 'example',
//!   { instanceType: 'instanceType', roleArn: 'roleArn' }); // Sensitive
//! // Compliant adds: kmsKeyId: encryptionKey.keyId
//! ```
//!
//! ## Construct-gated, zero-FP absence detection
//!
//! This port flags a `NewExpression` only when BOTH conditions hold:
//!
//! 1. The callee (after `get_inner_expression` strips wrapping
//!    parentheses/non-null assertions) names the distinctive
//!    `CfnNotebookInstance` construct — a bare identifier `CfnNotebookInstance`
//!    or a static member expression whose terminal property is
//!    `CfnNotebookInstance` (e.g. `sagemaker.CfnNotebookInstance`).
//! 2. NONE of the object-literal arguments contains a property whose key (a
//!    static identifier or a string literal) is exactly `kmsKeyId`.
//!
//! `CfnNotebookInstance` is a SageMaker-specific CDK class name distinctive
//! enough that gating on it makes this rule effectively false-positive free —
//! mirroring how `aws-iam-public-access` gates on the distinctive `AnyPrincipal`
//! name. Reporting the construct when `kmsKeyId` is absent is exactly the
//! documented Noncompliant behaviour. When a `kmsKeyId` property is present in
//! any object argument the notebook is encrypted and nothing is reported, and no
//! other construct name is matched. The new-expression span is reported.
//!
//! ## Flagged
//! ```js
//! new CfnNotebookInstance(this, 'x', { instanceType: 't', roleArn: 'r' });
//! new sagemaker.CfnNotebookInstance(this, 'x', {});
//! new CfnNotebookInstance(this, 'x');
//! ```
//!
//! ## Not flagged
//! ```js
//! new CfnNotebookInstance(this, 'x', { kmsKeyId: key.keyId }); // encrypted
//! new sagemaker.CfnNotebookInstance(this, 'x', { kmsKeyId: 'k' });
//! new Foo(this, 'x', {}); // wrong construct
//! ```

use oxc_ast::ast::{Argument, Expression, NewExpression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-sagemaker-unencrypted-notebook";

impl<'a> Scanner<'a> {
    /// Reports a `new CfnNotebookInstance(...)` created without a `kmsKeyId`
    /// property, leaving the SageMaker notebook's data unencrypted at rest.
    pub(crate) fn check_aws_sagemaker_unencrypted_notebook(&mut self, it: &NewExpression<'a>) {
        let is_notebook_instance = match it.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident.name.as_str() == "CfnNotebookInstance",
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "CfnNotebookInstance"
            }
            _ => false,
        };
        if !is_notebook_instance {
            return;
        }
        let has_kms_key = it.arguments.iter().any(|argument| {
            let Argument::ObjectExpression(options) = argument else {
                return false;
            };
            options.properties.iter().any(|property| {
                let ObjectPropertyKind::ObjectProperty(prop) = property else {
                    return false;
                };
                let key = match &prop.key {
                    PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                    PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                    _ => return false,
                };
                key == "kmsKeyId"
            })
        });
        if !has_kms_key {
            self.report(RULE_NAME, "sagemakerUnencryptedNotebook", it.span);
        }
    }
}
