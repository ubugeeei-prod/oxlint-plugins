//! Rule `aws-opensearchservice-domain` (SonarJS key S6308).
//!
//! Clean-room port from public RSPEC S6308 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Using unencrypted Elasticsearch/OpenSearch domains is security-sensitive.
//! Encryption at rest protects the domain's data from unauthorized access. In
//! AWS CDK this is configured via `encryptionAtRest: { enabled: true }` on the
//! high-level `Domain` construct, or `encryptionAtRestOptions: { enabled: true }`
//! on the low-level `CfnDomain`. The explicit insecure form sets
//! `enabled: false`, which disables encryption at rest.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `encryptionAtRest` or `encryptionAtRestOptions`
//! and whose value (after unwrapping parentheses) is an object expression that
//! contains a property with key `enabled` whose value is the boolean literal
//! `false`. The distinctive nested shape `encryptionAtRest: { enabled: false }`
//! is the explicit disable-encryption form, so flagging only this shape is
//! effectively zero-false-positive; no construct gating is needed. The outer
//! `encryptionAtRest`/`encryptionAtRestOptions` property span is reported.
//!
//! ## Deliberate under-report
//!
//! Like the sibling AWS encryption rules, this port deliberately does NOT flag
//! the absence or omission of encryption (a `Domain` created with no
//! `encryptionAtRest` option at all). Detecting omission would require
//! construct-type resolution and risk false positives, so it is intentionally
//! out of scope.
//!
//! ## Flagged
//! ```js
//! new Domain(this, 'd', { encryptionAtRest: { enabled: false } });
//! new CfnDomain(this, 'd', { encryptionAtRestOptions: { enabled: false } });
//! ```
//!
//! ## Not Flagged
//! ```js
//! new Domain(this, 'd', { encryptionAtRest: { enabled: true } }); // encrypted
//! new Domain(this, 'd', { encryptionAtRest: { enabled: flag } });  // non-literal
//! const x = { enabled: false };          // bare, not nested under the key
//! const x = { otherOption: { enabled: false } }; // different outer key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-opensearchservice-domain";

impl Scanner<'_> {
    pub(crate) fn check_aws_opensearchservice_domain_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "encryptionAtRest" && key != "encryptionAtRestOptions" {
            return;
        }
        let Expression::ObjectExpression(object) = it.value.get_inner_expression() else {
            return;
        };
        let disables_encryption = object.properties.iter().any(|prop| {
            let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                return false;
            };
            let inner_key = match &prop.key {
                PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                _ => return false,
            };
            if inner_key != "enabled" {
                return false;
            }
            matches!(&prop.value, Expression::BooleanLiteral(b) if !b.value)
        });
        if !disables_encryption {
            return;
        }
        self.report(RULE_NAME, "opensearchUnencrypted", it.span);
    }
}
