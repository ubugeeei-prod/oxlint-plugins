//! Rule `aws-ec2-unencrypted-ebs-volume` (SonarJS key S6275).
//!
//! Clean-room port from the public RSPEC S6275 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An AWS CDK EBS volume created without encryption leaves data-at-rest
//! unprotected: an adversary with access to the underlying storage media could
//! read sensitive contents. The CDK exposes this via the `Volume` construct,
//! whose `encrypted` option toggles at-rest encryption. Constructing a `Volume`
//! with `encrypted: false` explicitly disables that protection.
//!
//! ## Zero-FP subset (construct-gated)
//!
//! This port flags a `NewExpression` whose callee (after stripping wrapping
//! parentheses/non-null assertions via `get_inner_expression`) is the
//! distinctive `Volume` construct — either a bare identifier `Volume` or a
//! static member expression ending in `.Volume` (e.g. `ec2.Volume`) — AND one
//! of whose arguments is an object literal containing a property whose key
//! (static identifier or string literal) is exactly `encrypted` with the
//! boolean literal value `false`. The new-expression span is reported.
//!
//! Gating on the `Volume` construct name is what keeps this rule distinct from
//! `aws-efs-unencrypted` (which also keys off `encrypted: false`): only an EBS
//! `Volume` is matched here, so the two rules never collide and neither fires
//! on the other's construct. This combination is specific enough to the AWS EC2
//! CDK API to stay effectively false-positive free.
//!
//! The implicit form (`new Volume(...)` with no `encrypted` property) is
//! deliberately out of scope: flagging an absent property is far more prone to
//! false positives, so only the explicit `encrypted: false` shape is reported.
//!
//! ## Flagged
//! ```js
//! new ec2.Volume(this, 'v', { encrypted: false });
//! new Volume(this, 'v', { encrypted: false, size: x });
//! ```
//!
//! ## Not flagged
//! ```js
//! new ec2.Volume(this, 'v', { encrypted: true });   // encryption enabled
//! new Volume(this, 'v', {});                         // absent (out of scope)
//! new Volume(this, 'v');                             // no options object
//! new FileSystem(this, 'f', { encrypted: false });   // wrong construct (EFS)
//! ```

use oxc_ast::ast::{Argument, Expression, NewExpression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-ec2-unencrypted-ebs-volume";

impl<'a> Scanner<'a> {
    /// Reports a `new ec2.Volume(..., { encrypted: false })` whose explicit
    /// `encrypted: false` option leaves the EBS volume's data unencrypted.
    pub(crate) fn check_aws_ec2_unencrypted_ebs_volume(&mut self, expr: &NewExpression<'a>) {
        let is_volume = match expr.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident.name.as_str() == "Volume",
            Expression::StaticMemberExpression(member) => member.property.name.as_str() == "Volume",
            _ => false,
        };
        if !is_volume {
            return;
        }
        let has_encrypted_false = expr.arguments.iter().any(|argument| {
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
                matches!(&prop.value, Expression::BooleanLiteral(b) if !b.value)
            })
        });
        if has_encrypted_false {
            self.report(RULE_NAME, "ebsUnencrypted", expr.span);
        }
    }
}
