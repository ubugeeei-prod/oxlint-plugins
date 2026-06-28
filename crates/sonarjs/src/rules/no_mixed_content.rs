//! Rule `no-mixed-content` (SonarJS key S5730).
//!
//! Clean-room port from public RSPEC S5730 documentation and general knowledge
//! of Content Security Policy only; no upstream eslint-plugin-sonarjs source,
//! tests, fixtures, or message strings were consulted or copied.
//!
//! "Mixed content" is the result of loading a page over HTTPS while some of its
//! sub-resources (images, scripts, stylesheets, ...) are fetched over plain
//! HTTP. The recommended mitigation, when configuring a Content Security Policy
//! via the `helmet` middleware, is to add the `block-all-mixed-content`
//! directive to the policy's `directives` object so the browser refuses to load
//! the insecure resources.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key is exactly `directives` and
//! whose value is an object literal that:
//!   1. contains at least one well-known CSP fetch/navigation directive key
//!      (e.g. `default-src`, `script-src`, `frame-ancestors`, ...), which marks
//!      the object unambiguously as a helmet CSP `directives` configuration, and
//!   2. does NOT contain a `block-all-mixed-content` directive (in either the
//!      kebab-case `"block-all-mixed-content"` or helmet's camelCase
//!      `blockAllMixedContent` spelling).
//!
//! Requiring a recognised CSP directive before reporting keeps the check
//! effectively false-positive-free: a `directives` object holding `default-src`
//! is unmistakably a CSP policy. This covers both the
//! `helmet.contentSecurityPolicy({ directives: { ... } })` call form and the
//! `helmet({ contentSecurityPolicy: { directives: { ... } } })` option form,
//! because the `directives` property is visited in both. Empty or non-CSP
//! `directives` objects are intentionally not reported (under-report rather than
//! over-report). The `directives` property span is reported.
//!
//! ## Flagged
//! ```js
//! helmet.contentSecurityPolicy({
//!   directives: { "default-src": ["'self'"] } // missing block-all-mixed-content
//! });
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet.contentSecurityPolicy({
//!   directives: { "default-src": ["'self'"], "block-all-mixed-content": [] }
//! });
//! helmet.contentSecurityPolicy({
//!   directives: { defaultSrc: ["'self'"], blockAllMixedContent: [] }
//! });
//! const x = { directives: { foo: 1 } }; // not a CSP directives object
//! ```

use oxc_ast::ast::{Expression, ObjectExpression, ObjectProperty, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-mixed-content";

fn property_key_name<'a>(key: &'a PropertyKey<'_>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(ident.name.as_str()),
        PropertyKey::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

/// True for keys that identify a `block-all-mixed-content` CSP directive, in
/// either the standard kebab-case or helmet's camelCase spelling.
fn is_block_all_mixed_content(key: &str) -> bool {
    matches!(key, "block-all-mixed-content" | "blockAllMixedContent")
}

/// True for keys that are recognisable CSP fetch/navigation/document directives,
/// in either kebab-case (CSP header spelling) or helmet camelCase spelling. Used
/// purely as a "this is really a CSP directives object" guard.
fn is_known_csp_directive(key: &str) -> bool {
    matches!(
        key,
        "default-src"
            | "defaultSrc"
            | "script-src"
            | "scriptSrc"
            | "style-src"
            | "styleSrc"
            | "img-src"
            | "imgSrc"
            | "connect-src"
            | "connectSrc"
            | "font-src"
            | "fontSrc"
            | "object-src"
            | "objectSrc"
            | "media-src"
            | "mediaSrc"
            | "frame-src"
            | "frameSrc"
            | "child-src"
            | "childSrc"
            | "worker-src"
            | "workerSrc"
            | "manifest-src"
            | "manifestSrc"
            | "base-uri"
            | "baseUri"
            | "form-action"
            | "formAction"
            | "frame-ancestors"
            | "frameAncestors"
            | "upgrade-insecure-requests"
            | "upgradeInsecureRequests"
    )
}

impl Scanner<'_> {
    pub(crate) fn check_no_mixed_content(&mut self, it: &ObjectProperty<'_>) {
        let Some(key) = property_key_name(&it.key) else {
            return;
        };
        if key != "directives" {
            return;
        }
        let Expression::ObjectExpression(directives) = &it.value else {
            return;
        };
        let directives: &ObjectExpression<'_> = directives;

        let mut has_known_directive = false;
        let mut has_block_all = false;
        for prop in &directives.properties {
            let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                continue;
            };
            let Some(inner_key) = property_key_name(&prop.key) else {
                continue;
            };
            if is_block_all_mixed_content(inner_key) {
                has_block_all = true;
            }
            if is_known_csp_directive(inner_key) {
                has_known_directive = true;
            }
        }

        if has_known_directive && !has_block_all {
            self.report(RULE_NAME, "addBlockAllMixedContent", it.span);
        }
    }
}
