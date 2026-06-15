//! Rule `content-security-policy` (SonarJS key S5728).
//!
//! Clean-room port from public RSPEC S5728 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! A Content Security Policy (CSP) is an important defense-in-depth measure
//! against Cross-Site Scripting (XSS) and other injection attacks: it tells
//! the browser which origins are allowed to load resources. The `helmet`
//! middleware enables CSP by default, so explicitly turning it off with
//! `helmet({ contentSecurityPolicy: false })` removes that protection.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `contentSecurityPolicy` and whose value is the
//! boolean literal `false`. The `contentSecurityPolicy` key is distinctive to
//! helmet configuration objects, so flagging only `contentSecurityPolicy:
//! false` is effectively zero-false-positive in practice. The property span is
//! reported.
//!
//! ## Flagged
//! ```js
//! helmet({ contentSecurityPolicy: false }); // CSP explicitly disabled
//! const x = { contentSecurityPolicy: false };
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet({ contentSecurityPolicy: true });  // explicitly enabled
//! const x = { contentSecurityPolicy: opts }; // non-literal value
//! const x = { csp: false };                  // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "content-security-policy";

impl Scanner<'_> {
    pub(crate) fn check_content_security_policy_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "contentSecurityPolicy" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "contentSecurityPolicy", it.span);
    }
}
