//! Rule `no-angular-bypass-sanitization` (SonarJS key S6268).
//!
//! Clean-room port. Angular treats all values as untrusted by default and
//! sanitizes them automatically to prevent cross-site scripting. The
//! `DomSanitizer` API exposes `bypassSecurityTrust*` methods that disable this
//! built-in protection for a given value; trusting attacker-controlled data
//! this way reintroduces an XSS vulnerability. This implements the
//! zero-false-positive subset: a call to one of the `bypassSecurityTrust*`
//! methods, whose names are essentially unique to Angular's `DomSanitizer`.
//!
//! **Flagged** — a `CallExpression` whose callee (after unwrapping
//! parentheses) is a `StaticMemberExpression` whose property name is one of:
//! - `bypassSecurityTrustHtml`
//! - `bypassSecurityTrustStyle`
//! - `bypassSecurityTrustScript`
//! - `bypassSecurityTrustUrl`
//! - `bypassSecurityTrustResourceUrl`
//!
//! The receiver's type is irrelevant; these method names are distinctive
//! enough that any call to one is treated as a DomSanitizer bypass, e.g.
//! `this.sanitizer.bypassSecurityTrustHtml(html)` or
//! `ds.bypassSecurityTrustResourceUrl(url)`.
//!
//! **Not flagged**:
//! - `this.sanitizer.sanitize(x)` — an unrelated DomSanitizer method.
//! - `foo.bypassOther(x)` — a method whose name is not in the bypass set.
//! - `bypassSecurityTrustHtml` (no call) — a property/identifier reference
//!   without invocation.
//!
//! Behaviour is reproduced from the public RSPEC S6268 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-angular-bypass-sanitization";

fn is_bypass_method(name: &str) -> bool {
    matches!(
        name,
        "bypassSecurityTrustHtml"
            | "bypassSecurityTrustStyle"
            | "bypassSecurityTrustScript"
            | "bypassSecurityTrustUrl"
            | "bypassSecurityTrustResourceUrl"
    )
}

impl Scanner<'_> {
    pub(crate) fn check_no_angular_bypass_sanitization(&mut self, call: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        if is_bypass_method(member.property.name.as_str()) {
            self.report(RULE_NAME, "angularBypassSanitization", call.span);
        }
    }
}
