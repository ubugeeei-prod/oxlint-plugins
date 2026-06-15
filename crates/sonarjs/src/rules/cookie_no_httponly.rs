//! Rule `cookie-no-httponly` (SonarJS key S3330).
//!
//! Clean-room port from public RSPEC S3330 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! A cookie configured with `httpOnly: false` is accessible to client-side
//! JavaScript, which lets a Cross-Site Scripting (XSS) flaw read the cookie
//! (e.g. steal a session). The `HttpOnly` flag should be `true` so the cookie
//! is hidden from scripts.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `httpOnly` and whose value is the boolean
//! literal `false`. The camelCase `httpOnly` key is distinctive to cookie /
//! session configuration objects, so flagging only `httpOnly: false` is
//! effectively zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! const c = { httpOnly: false };              // direct config literal
//! session({ cookie: { httpOnly: false } });   // nested cookie config
//! cookies.set('x', v, { httpOnly: false });   // call-argument config
//! ```
//!
//! ## Not Flagged
//! ```js
//! const c = { httpOnly: true };   // explicitly secure
//! const c = { httpOnly: x };      // non-literal value (cannot prove unsafe)
//! const c = { secure: false };    // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "cookie-no-httponly";

impl Scanner<'_> {
    pub(crate) fn check_cookie_no_httponly_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "httpOnly" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "cookieNoHttpOnly", it.span);
    }
}
