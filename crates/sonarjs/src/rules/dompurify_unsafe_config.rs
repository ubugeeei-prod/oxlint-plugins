//! Rule `dompurify-unsafe-config` (SonarJS key S8479).
//!
//! Clean-room port from the public RSPEC S8479 metadata ("DOMPurify
//! configuration should not be bypassable", CWE-79 XSS / CWE-183 permissive
//! allowlist) and public DOMPurify documentation only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.
//!
//! DOMPurify sanitizes untrusted HTML to prevent Cross-Site Scripting. Some of
//! its configuration options weaken or completely bypass that protection.
//! Passing such an option re-opens the XSS hole the sanitizer was meant to
//! close.
//!
//! ## Zero-FP subset
//!
//! Reproducing the full rule the way SonarJS does relies on type information to
//! recognise that a given object literal is actually a DOMPurify configuration
//! argument. The runtime here has no type checker, so this port keys on a
//! property name that is *verbatim distinctive* to DOMPurify configuration and
//! whose dangerous value is unambiguous:
//!
//! - `ALLOW_UNKNOWN_PROTOCOLS: true`
//!
//! `ALLOW_UNKNOWN_PROTOCOLS` is a DOMPurify-specific option; enabling it lets
//! arbitrary URI protocols (for example `javascript:` or `data:`) survive
//! sanitization, defeating DOMPurify's URL filtering. Because the all-caps key
//! is effectively never used outside DOMPurify configuration, flagging only
//! `ALLOW_UNKNOWN_PROTOCOLS: true` is effectively zero-false-positive. The
//! property span is reported.
//!
//! ## Flagged
//! ```js
//! DOMPurify.sanitize(dirty, { ALLOW_UNKNOWN_PROTOCOLS: true });
//! const cfg = { ALLOW_UNKNOWN_PROTOCOLS: true };
//! ```
//!
//! ## Not flagged
//! ```js
//! DOMPurify.sanitize(dirty, { ALLOW_UNKNOWN_PROTOCOLS: false }); // safe value
//! DOMPurify.sanitize(dirty, { ADD_TAGS: ["b"] });                // other key
//! const cfg = { ALLOW_UNKNOWN_PROTOCOLS: flag };                 // non-literal
//! ```
//!
//! Other bypassing options (`ADD_TAGS` / `ADD_ATTR` with dangerous values,
//! `SANITIZE_DOM: false`, etc.) are a documented follow-up: they either need
//! value-set analysis to stay false-positive-free or are far less distinctive,
//! so they are intentionally out of scope for this narrow port.

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "dompurify-unsafe-config";

impl Scanner<'_> {
    pub(crate) fn check_dompurify_unsafe_config(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "ALLOW_UNKNOWN_PROTOCOLS" {
            return;
        }
        let is_true = matches!(&it.value, Expression::BooleanLiteral(b) if b.value);
        if !is_true {
            return;
        }
        self.report(RULE_NAME, "unsafeConfig", it.span);
    }
}
