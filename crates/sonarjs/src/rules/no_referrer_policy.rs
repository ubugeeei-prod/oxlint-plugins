//! Rule `no-referrer-policy` (SonarJS key S5736).
//!
//! Clean-room port from public RSPEC S5736 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! The HTTP `Referer` header set by the browser exposes the originating URL to
//! other origins. When that URL carries confidential data (query parameters,
//! session tokens, etc.), a weak `Referrer-Policy` leaks it to third parties.
//! Configuring helmet's `referrerPolicy` with `'no-referrer-when-downgrade'`
//! or `'unsafe-url'` sends the full URL across origins and is unsafe; a strict
//! value such as `'no-referrer'` or `'same-origin'` should be used instead.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `policy` and whose value is a string literal
//! equal to one of the leaky `Referrer-Policy` values: `'no-referrer-when-
//! downgrade'` or `'unsafe-url'`. While `policy` is a generic key on its own,
//! the gating is the specific string value: those exact `Referrer-Policy`
//! tokens are distinctive to referrer-policy configuration, so flagging only
//! `policy: 'no-referrer-when-downgrade'` / `policy: 'unsafe-url'` is
//! effectively zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! helmet.referrerPolicy({ policy: 'no-referrer-when-downgrade' }); // leaky
//! helmet.referrerPolicy({ policy: 'unsafe-url' });                 // leaky
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet.referrerPolicy({ policy: 'no-referrer' });  // strict, safe
//! helmet.referrerPolicy({ policy: 'same-origin' });  // strict, safe
//! const c = { policy: x };                           // non-literal value
//! const c = { other: 'unsafe-url' };                 // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-referrer-policy";

/// `Referrer-Policy` values that cause the full URL to be sent to other
/// origins and therefore leak any confidential information it contains.
const LEAKY_POLICIES: [&str; 2] = ["no-referrer-when-downgrade", "unsafe-url"];

impl Scanner<'_> {
    pub(crate) fn check_no_referrer_policy_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "policy" {
            return;
        }
        let Expression::StringLiteral(value) = &it.value else {
            return;
        };
        if !LEAKY_POLICIES.contains(&value.value.as_str()) {
            return;
        }
        self.report(RULE_NAME, "noReferrerPolicy", it.span);
    }
}
