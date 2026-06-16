//! Rule `aws-efs-unencrypted` (SonarJS key S6332).
//!
//! Clean-room port from the public RSPEC S6332 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Amazon EFS file systems can transparently encrypt data at rest. When that
//! encryption is explicitly disabled, sensitive data on the underlying storage
//! is left in the clear and exposed should an adversary gain access to the
//! media. In the AWS CDK this surfaces as an `efs.FileSystem` construct created
//! with `encrypted: false`.
//!
//! ## Construct-gated, zero-FP subset
//!
//! This port flags a `NewExpression` only when BOTH conditions hold:
//!
//! 1. The callee (after `get_inner_expression` strips wrapping
//!    parentheses/non-null assertions) names the `FileSystem` construct — a bare
//!    identifier `FileSystem` or a static member expression whose terminal
//!    property is `FileSystem` (e.g. `efs.FileSystem`).
//! 2. One of the arguments is an object literal containing a property whose key
//!    (a static identifier or a string literal) is exactly `encrypted` and whose
//!    value is the boolean literal `false`.
//!
//! Gating on the `FileSystem` construct name is what keeps this rule distinct
//! from the EBS-volume rule, which also keys off `encrypted: false`: only an EFS
//! `FileSystem` is reported here. The new-expression span is reported.
//!
//! Encryption disabled by default via the lower-level `CfnFileSystem` construct
//! (an absence, not an explicit `encrypted: false`) is deliberately out of scope
//! to stay false-positive free.
//!
//! ## Flagged
//! ```js
//! new efs.FileSystem(this, 'f', { encrypted: false });
//! new FileSystem(this, 'f', { encrypted: false, vpc: v });
//! ```
//!
//! ## Not flagged
//! ```js
//! new efs.FileSystem(this, 'f', { encrypted: true }); // encryption enabled
//! new FileSystem(this, 'f', {});                       // absence, out of scope
//! new Volume(this, 'v', { encrypted: false });         // wrong construct
//! new FileSystem(this, 'f');                            // no options object
//! ```

use oxc_ast::ast::{Argument, Expression, NewExpression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-efs-unencrypted";

impl<'a> Scanner<'a> {
    /// Reports a `new efs.FileSystem(..., { encrypted: false })` whose EFS file
    /// system is created without encryption at rest.
    pub(crate) fn check_aws_efs_unencrypted(&mut self, expr: &NewExpression<'a>) {
        let is_file_system = match expr.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident.name.as_str() == "FileSystem",
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "FileSystem"
            }
            _ => false,
        };
        if !is_file_system {
            return;
        }
        let disables_encryption = expr.arguments.iter().any(|argument| {
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
                if key != "encrypted" {
                    return false;
                }
                matches!(&prop.value, Expression::BooleanLiteral(lit) if !lit.value)
            })
        });
        if disables_encryption {
            self.report(RULE_NAME, "efsUnencrypted", expr.span);
        }
    }
}
