//! Rule `no-intrusive-permissions` (SonarJS key S5604).
//!
//! Clean-room port. Requesting a powerful browser permission — geolocation or
//! desktop notifications — is a security hotspot: such permissions expose
//! sensitive user data or capabilities, so each request should be deliberate
//! and justified. This implements ONLY the unambiguous, zero-false-positive
//! subset: direct calls to a small set of permission-request APIs whose member
//! chains are essentially unique to those APIs. The check is purely syntactic
//! (no scope analysis), so the distinctive object-then-property shape is what
//! guarantees the absence of false positives.
//!
//! **Flagged** — a `CallExpression` whose callee (after unwrapping
//! parentheses) is a `StaticMemberExpression` matching one of these chains:
//! - `navigator.geolocation.getCurrentPosition(...)` — property
//!   `getCurrentPosition` on the `navigator.geolocation` object.
//! - `navigator.geolocation.watchPosition(...)` — property `watchPosition` on
//!   the `navigator.geolocation` object.
//! - `Notification.requestPermission(...)` — property `requestPermission` on
//!   the `Notification` identifier.
//! - `navigator.permissions.query(...)` — property `query` on the
//!   `navigator.permissions` object.
//!
//! **Not flagged**:
//! - `navigator.geolocation` / `navigator.userAgent` — a bare member access
//!   with no call is not a permission request.
//! - `foo.getCurrentPosition()` — the property name matches but the object
//!   chain does not (`foo` is not `navigator.geolocation`).
//! - other `navigator.*` or `Notification.*` methods — only the four chains
//!   above are recognised.
//!
//! No scope/shadowing analysis is performed: the `navigator` / `Notification`
//! base identifiers are matched syntactically. Because the full member chains
//! are so distinctive, locally shadowing those names with an object exposing
//! the same chain is implausible in real code, so this rule stays zero-FP in
//! practice and is NOT added to the `needs_semantic` gate.
//!
//! Behaviour is reproduced from the public RSPEC S5604 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-intrusive-permissions";

impl Scanner<'_> {
    pub(crate) fn check_no_intrusive_permissions(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        let property = member.property.name.as_str();
        let object = member.object.get_inner_expression();

        let is_intrusive = match property {
            "getCurrentPosition" | "watchPosition" => is_navigator_member(object, "geolocation"),
            "query" => is_navigator_member(object, "permissions"),
            "requestPermission" => matches!(
                object,
                Expression::Identifier(ident) if ident.name == "Notification"
            ),
            _ => false,
        };

        if is_intrusive {
            self.report(RULE_NAME, "intrusivePermission", expr.span);
        }
    }
}

/// Returns `true` when `object` is the static member expression
/// `navigator.<property>` (object `navigator` identifier, given property name).
fn is_navigator_member(object: &Expression<'_>, property: &str) -> bool {
    let Expression::StaticMemberExpression(inner) = object else {
        return false;
    };
    if inner.property.name != property {
        return false;
    }
    matches!(
        inner.object.get_inner_expression(),
        Expression::Identifier(ident) if ident.name == "navigator"
    )
}
