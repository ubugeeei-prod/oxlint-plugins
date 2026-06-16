//! Rule `frame-ancestors` (SonarJS key S5732).
//!
//! Clean-room port from public RSPEC S5732 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! A Content Security Policy `frame-ancestors` directive controls which origins
//! may embed a page in a frame, and is the modern defense against clickjacking.
//! The RSPEC marks setting this directive to `'none'` as security-sensitive: it
//! disables framing entirely, which can be intentional but is worth a manual
//! review because it is a frequent misconfiguration when a site actually needs
//! to allow specific trusted ancestors.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or string
//! literal) is exactly `frameAncestors` AND whose value is an array literal
//! containing at least one string-literal element whose parsed value is exactly
//! `'none'` (the six characters `'`, `n`, `o`, `n`, `e`, `'` — the CSP keyword
//! is itself quoted, so the source token is `"'none'"`). This mirrors the
//! documented Noncompliant pattern of the `helmet.contentSecurityPolicy`
//! `directives` configuration. The distinctive `frameAncestors` key combined
//! with the quoted CSP keyword `'none'` makes this effectively
//! zero-false-positive. The `ObjectProperty` span is reported.
//!
//! ## Flagged
//! ```js
//! helmet.contentSecurityPolicy({
//!   directives: {
//!     frameAncestors: ["'none'"], // disables framing entirely
//!   },
//! });
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet.contentSecurityPolicy({
//!   directives: {
//!     frameAncestors: ["'example.com'"], // a specific trusted origin
//!   },
//! });
//! const o = { frameAncestors: "'none'" }; // value is not an array
//! const o = { other: ["'none'"] };        // key is not frameAncestors
//! ```

use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "frame-ancestors";

/// The quoted CSP keyword that disables framing entirely. The directive value
/// is itself quoted, so the parsed string is the six characters `'none'`.
const CSP_NONE: &str = "'none'";

impl Scanner<'_> {
    pub(crate) fn check_frame_ancestors_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "frameAncestors" {
            return;
        }
        let Expression::ArrayExpression(array) = it.value.get_inner_expression() else {
            return;
        };
        let disables_framing = array.elements.iter().any(|element| match element {
            ArrayExpressionElement::StringLiteral(lit) => lit.value.as_str() == CSP_NONE,
            _ => false,
        });
        if !disables_framing {
            return;
        }
        self.report(RULE_NAME, "frameAncestors", it.span);
    }
}
