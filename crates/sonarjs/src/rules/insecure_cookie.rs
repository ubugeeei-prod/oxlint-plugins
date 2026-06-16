//! Rule `insecure-cookie` (SonarJS key S2092).
//!
//! Clean-room port from public RSPEC S2092 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! A cookie configured with `secure: false` is sent by the browser over
//! unencrypted HTTP requests, where it can be observed by an attacker during a
//! man-in-the-middle attack. The `secure` flag should be `true` so the cookie
//! is only transmitted over HTTPS.
//!
//! ## Zero-FP sibling-gated subset
//!
//! The bare key `secure` is far too generic to flag on its own: TLS sockets,
//! database drivers, and HTTP clients (e.g. axios) all use a `secure` option
//! that is unrelated to cookies. To stay zero-false-positive without
//! call-context analysis, this port flags a `secure: false` property ONLY when
//! the enclosing object literal ALSO contains at least one distinctive
//! cookie-marker sibling key: `httpOnly`, `sameSite`, `maxAge`, `domain`,
//! `path`, or `signed`. The presence of such a sibling makes it overwhelmingly
//! likely that the object is a cookie configuration. The span of the
//! `secure: false` property is reported.
//!
//! ## Flagged
//! ```js
//! const c = { secure: false, httpOnly: true };          // cookie config literal
//! session({ cookie: { secure: false, sameSite: 'lax' } }); // nested cookie config
//! ```
//!
//! ## Not Flagged
//! ```js
//! const c = { secure: false };                       // no cookie-marker sibling
//! const c = { secure: true, httpOnly: true };        // explicitly secure
//! const tls = { secure: false, rejectUnauthorized: false }; // not a cookie object
//! const c = { secure: x, maxAge: 1 };                // non-literal value
//! ```

use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind, PropertyKey};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "insecure-cookie";

/// Returns the static key name of a property (static identifier or string
/// literal key), or `None` for computed/other keys.
fn property_key_name<'a>(key: &'a PropertyKey<'_>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(ident.name.as_str()),
        PropertyKey::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

const COOKIE_MARKER_KEYS: [&str; 6] =
    ["httpOnly", "sameSite", "maxAge", "domain", "path", "signed"];

impl Scanner<'_> {
    pub(crate) fn check_insecure_cookie(&mut self, it: &ObjectExpression<'_>) {
        let mut insecure_span: Option<Span> = None;
        let mut has_cookie_marker = false;

        for prop in &it.properties {
            let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                continue;
            };
            let Some(name) = property_key_name(&prop.key) else {
                continue;
            };
            if name == "secure" {
                if matches!(&prop.value, Expression::BooleanLiteral(b) if !b.value) {
                    insecure_span = Some(prop.span);
                }
            } else if COOKIE_MARKER_KEYS.contains(&name) {
                has_cookie_marker = true;
            }
        }

        if !has_cookie_marker {
            return;
        }
        let Some(span) = insecure_span else {
            return;
        };
        self.report(RULE_NAME, "insecureCookie", span);
    }
}
