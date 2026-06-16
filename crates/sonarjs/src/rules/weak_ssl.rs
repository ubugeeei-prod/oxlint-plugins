//! Rule `weak-ssl` (SonarJS key S4423).
//!
//! Clean-room port from public RSPEC S4423 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Configuring a TLS/SSL client or server with a weak (deprecated) protocol
//! version exposes the connection to known cryptographic weaknesses. SSL v2/v3
//! and TLS v1.0/v1.1 are deprecated and must not be used; TLS v1.2 or higher
//! should be configured instead.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` (the shape of a Node.js `tls`/`https`
//! options object) whose key is a static identifier or string literal and:
//!
//! - the key is `secureProtocol` with a string-literal value in the weak set
//!   {`TLSv1_method`, `TLSv1_1_method`, `SSLv2_method`, `SSLv3_method`,
//!   `SSLv23_method`}; or
//! - the key is `minVersion` or `maxVersion` with a string-literal value in
//!   {`TLSv1`, `TLSv1.1`}.
//!
//! These key/value combinations are the distinctive constants used by the
//! Node.js TLS API to select a protocol version, so flagging only these exact
//! weak string values is effectively zero-false-positive. The property span is
//! reported.
//!
//! ## Flagged
//! ```js
//! const o = { secureProtocol: 'TLSv1_method' };   // weak protocol method
//! const o = { minVersion: 'TLSv1.1' };            // weak minimum version
//! const o = { maxVersion: 'TLSv1' };              // weak maximum version
//! ```
//!
//! ## Not Flagged
//! ```js
//! const o = { secureProtocol: 'TLSv1_2_method' }; // strong protocol method
//! const o = { minVersion: 'TLSv1.2' };            // strong minimum version
//! const o = { secureProtocol: x };                // non-literal value
//! const o = { other: 'TLSv1_method' };            // unrelated key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "weak-ssl";

/// Weak `secureProtocol` method constants (deprecated SSL/TLS versions).
const WEAK_SECURE_PROTOCOLS: [&str; 5] = [
    "TLSv1_method",
    "TLSv1_1_method",
    "SSLv2_method",
    "SSLv3_method",
    "SSLv23_method",
];

/// Weak `minVersion`/`maxVersion` values (deprecated TLS versions).
const WEAK_VERSIONS: [&str; 2] = ["TLSv1", "TLSv1.1"];

impl Scanner<'_> {
    pub(crate) fn check_weak_ssl_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        let Expression::StringLiteral(value) = &it.value else {
            return;
        };
        let value = value.value.as_str();
        let is_weak = match key {
            "secureProtocol" => WEAK_SECURE_PROTOCOLS.contains(&value),
            "minVersion" | "maxVersion" => WEAK_VERSIONS.contains(&value),
            _ => false,
        };
        if !is_weak {
            return;
        }
        self.report(RULE_NAME, "weakSsl", it.span);
    }
}
