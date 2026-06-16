//! Rule `aws-sns-unencrypted-topics` (SonarJS key S6327).
//!
//! Clean-room port from the public RSPEC S6327 documentation only; no upstream
//! source, tests, fixtures, helper code, or message strings were consulted or
//! copied. The RSPEC title is "Using unencrypted SNS topics is
//! security-sensitive": an AWS CDK SNS topic created without a KMS master key
//! is left unencrypted at rest, so sensitive messages could be exposed should
//! an adversary gain access to the underlying storage. This is a *security
//! hotspot* — a place that warrants human review — rather than a guaranteed
//! vulnerability.
//!
//! ## Construct-gated, zero-false-positive absence
//!
//! Unlike rules that key off an explicit `encrypted: false`, the SNS hotspot is
//! the *absence* of an encryption property. Flagging an absent property is only
//! safe when the construct itself is unmistakably an SNS topic, so this port
//! reports exactly the two construct shapes the public RSPEC demonstrates as
//! Noncompliant, each gated to stay false-positive free:
//!
//! 1. **Low-level `CfnTopic`** — a distinctive name. The callee (after
//!    `get_inner_expression` strips wrapping parentheses/non-null assertions) is
//!    either a bare identifier `CfnTopic` or a static member expression whose
//!    terminal property is `CfnTopic` (e.g. `sns.CfnTopic`). The construct is
//!    flagged when NO argument object literal carries a `kmsMasterKeyId`
//!    property (key as a static identifier or string literal). The `CfnTopic`
//!    name is specific enough to the AWS SNS CloudFormation API that it cannot
//!    reasonably collide with unrelated code.
//!
//! 2. **High-level `Topic`** — a *generic* name, so it must be receiver-gated.
//!    The callee is a static member expression whose terminal property is
//!    `Topic` AND whose object (after `get_inner_expression`) is the bare
//!    identifier `sns` — the conventional alias for the `aws-cdk-lib/aws-sns`
//!    import. Only `new sns.Topic(...)` matches; it is flagged when NO argument
//!    object literal carries a `masterKey` property.
//!
//! A bare `new Topic(...)` is deliberately NOT flagged: `Topic` is far too
//! generic a class name to assume an SNS topic, so matching it bare would
//! produce false positives. This is a documented, intentional under-report — we
//! accept missing the bare-identifier case in exchange for zero false
//! positives. Flagging the gated construct when the relevant encryption
//! property is absent is exactly the documented Noncompliant behaviour; when
//! the property IS present the construct is compliant and nothing is reported.
//!
//! ## Flagged
//! ```js
//! new CfnTopic(this, 'exampleCfnTopic');            // no kmsMasterKeyId
//! new sns.CfnTopic(this, 'x');                       // no kmsMasterKeyId
//! new sns.Topic(this, 'exampleTopic');               // no masterKey
//! ```
//!
//! ## Not flagged
//! ```js
//! new CfnTopic(this, 'x', { kmsMasterKeyId: key.keyId }); // encrypted
//! new sns.Topic(this, 'x', { masterKey: key });           // encrypted
//! new Topic(this, 'exampleTopic');                        // bare Topic — generic, deliberate under-report
//! new sns.Queue(this, 'q');                               // wrong construct
//! ```

use oxc_ast::ast::{
    Argument, Expression, NewExpression, ObjectExpression, ObjectPropertyKind, PropertyKey,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-sns-unencrypted-topics";

impl<'a> Scanner<'a> {
    /// Reports an AWS CDK SNS topic constructed without encryption: a
    /// `CfnTopic` lacking `kmsMasterKeyId`, or an `sns.Topic` lacking
    /// `masterKey`. Gated on the distinctive `CfnTopic` name and the `sns.Topic`
    /// receiver so the absence check stays false-positive free.
    pub(crate) fn check_aws_sns_unencrypted_topics(&mut self, expr: &NewExpression<'a>) {
        let callee = expr.callee.get_inner_expression();

        // (a) Low-level CfnTopic: bare `CfnTopic` or `<obj>.CfnTopic`.
        let is_cfn_topic = match callee {
            Expression::Identifier(ident) => ident.name.as_str() == "CfnTopic",
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "CfnTopic"
            }
            _ => false,
        };
        if is_cfn_topic {
            if !has_property(expr, "kmsMasterKeyId") {
                self.report(RULE_NAME, "snsUnencryptedTopic", expr.span);
            }
            return;
        }

        // (b) High-level Topic, receiver-gated to `sns.Topic` only (the `Topic`
        // name alone is too generic to match safely).
        let is_sns_topic = match callee {
            Expression::StaticMemberExpression(member) => {
                let property_is_topic = member.property.name.as_str() == "Topic";
                let object_is_sns = matches!(
                    member.object.get_inner_expression(),
                    Expression::Identifier(ident) if ident.name.as_str() == "sns"
                );
                property_is_topic && object_is_sns
            }
            _ => false,
        };
        if is_sns_topic && !has_property(expr, "masterKey") {
            self.report(RULE_NAME, "snsUnencryptedTopic", expr.span);
        }
    }
}

/// Returns `true` when any argument of the new-expression is an object literal
/// carrying a property whose key (static identifier or string literal) equals
/// `name`.
fn has_property(expr: &NewExpression<'_>, name: &str) -> bool {
    expr.arguments.iter().any(|argument| {
        let Argument::ObjectExpression(options) = argument else {
            return false;
        };
        object_has_key(options, name)
    })
}

fn object_has_key(options: &ObjectExpression<'_>, name: &str) -> bool {
    options.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(prop) = property else {
            return false;
        };
        let key = match &prop.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return false,
        };
        key == name
    })
}
