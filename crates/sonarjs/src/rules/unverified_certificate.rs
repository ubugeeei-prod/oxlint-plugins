//! Rule `unverified-certificate` (SonarJS key S4830).
//!
//! Clean-room port from public RSPEC S4830 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Disabling TLS server-certificate validation (by setting `rejectUnauthorized`
//! to `false` in a Node.js `https.request`, `tls.connect`, or `request` options
//! object) removes the trust check that proves a server is who it claims to be.
//! An attacker can then impersonate the server (man-in-the-middle) and read or
//! tamper with the encrypted traffic. Certificate validation should stay
//! enabled (`rejectUnauthorized: true`, or simply omit the option).
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `rejectUnauthorized` and whose value is the
//! boolean literal `false`. The `rejectUnauthorized` key is distinctive to
//! Node.js TLS configuration, so flagging only `rejectUnauthorized: false` is
//! effectively zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! https.request({ rejectUnauthorized: false });  // disables validation
//! const o = { rejectUnauthorized: false };        // TLS options literal
//! ```
//!
//! ## Not Flagged
//! ```js
//! const o = { rejectUnauthorized: true };  // validation kept on
//! const o = { rejectUnauthorized: x };     // non-literal value (cannot prove)
//! const o = { other: false };              // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "unverified-certificate";

impl Scanner<'_> {
    pub(crate) fn check_unverified_certificate_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "rejectUnauthorized" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "unverifiedCertificate", it.span);
    }
}
