//! Rule `no-ip-forward` (SonarJS key S5759).
//!
//! Clean-room port from public RSPEC S5759 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! HTTP proxies can be configured to forward the client IP address to the
//! upstream server via the `X-Forwarded-For` / `Forwarded` headers. In the
//! `http-proxy` (node-http-proxy) and `http-proxy-middleware` libraries this is
//! enabled with the `xfwd: true` configuration option. Because an IP address is
//! personal information, forwarding it unnecessarily impacts user privacy and
//! can also enable IP-based access-control bypass; it should only be enabled
//! when the application genuinely needs the client IP.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `xfwd` and whose value is the boolean literal
//! `true`. The `xfwd` key is distinctive to the http-proxy proxy-server /
//! proxy-middleware configuration objects, so flagging only `xfwd: true` is
//! effectively zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! createProxyServer({ target: t, xfwd: true });      // node-http-proxy
//! createProxyMiddleware({ target: t, xfwd: true });  // http-proxy-middleware
//! ```
//!
//! ## Not Flagged
//! ```js
//! createProxyServer({ target: t, xfwd: false });   // explicitly disabled
//! const o = { xfwd: x };                            // non-literal value
//! const o = { other: true };                        // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-ip-forward";

impl Scanner<'_> {
    pub(crate) fn check_no_ip_forward_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "xfwd" {
            return;
        }
        let is_true = matches!(&it.value, Expression::BooleanLiteral(b) if b.value);
        if !is_true {
            return;
        }
        self.report(RULE_NAME, "noIpForward", it.span);
    }
}
