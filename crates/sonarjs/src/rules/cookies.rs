//! Rule `cookies` (SonarJS key S2255, deprecated but still part of the plugin).
//!
//! Clean-room port from the public RSPEC S2255 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! "Writing cookies is security-sensitive." Cookies are stored client-side and
//! are readable by the browser (and, for non-`HttpOnly` cookies, by JavaScript),
//! so writing a cookie can leak sensitive data to an attacker who gains access
//! to the client. This is a *security hotspot*: the rule deliberately flags
//! every cookie-write so a reviewer can confirm that no sensitive data is being
//! persisted in a cookie. The public RSPEC lists three authoritative sensitive
//! examples — a NodeJS `res.setHeader('Set-Cookie', …)`, an ExpressJS
//! `res.cookie(name, value)`, and a browser `document.cookie = …` assignment —
//! and this port matches exactly those three distinctive cookie-write shapes,
//! which keeps it effectively false-positive free.
//!
//! ## Zero-FP subset (three distinctive cookie-write shapes)
//!
//! (a) An `AssignmentExpression` whose left-hand side is the static member
//!     expression `document.cookie` (object identifier `document`, property
//!     `cookie`) — the browser cookie write. The assignment span is reported.
//!
//! (b) A `CallExpression` whose callee (after `get_inner_expression`) is a
//!     static member expression whose property name is exactly `cookie` and
//!     which has at least one argument — the ExpressJS `res.cookie(name, value)`
//!     write. The call span is reported.
//!
//! (c) A `CallExpression` whose callee is a static member expression whose
//!     property name is exactly `setHeader` and whose first argument is a string
//!     literal equal (case-insensitively) to `Set-Cookie` — the NodeJS built-in
//!     write. The call span is reported.
//!
//! ## Deliberately NOT flagged
//!
//! Cookie *reads* are out of scope: a bare `document.cookie` reference that is
//! not the target of an assignment, and `req.cookies` reads, are not writes. A
//! `.cookie()` call with zero arguments is not a write (it does not set a value),
//! and a `setHeader` call with any other header name is unrelated.
//!
//! ## Flagged
//! ```js
//! document.cookie = "name=John";                      // (a) browser write
//! res.cookie('name', 'John');                          // (b) ExpressJS write
//! res.setHeader('Set-Cookie', ['type=ninja']);         // (c) NodeJS write
//! ```
//!
//! ## Not flagged
//! ```js
//! const c = document.cookie;          // read, not a write
//! const all = req.cookies;            // read, not a write
//! res.cookie();                       // zero-argument call, not a write
//! res.setHeader('Content-Type', 'x'); // different header name
//! ```

use oxc_ast::ast::{Argument, AssignmentExpression, AssignmentTarget, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "cookies";

impl Scanner<'_> {
    /// Shape (a): flags a `document.cookie = …` browser cookie write.
    pub(crate) fn check_cookies_assignment(&mut self, it: &AssignmentExpression<'_>) {
        let AssignmentTarget::StaticMemberExpression(member) = &it.left else {
            return;
        };
        if member.property.name.as_str() != "cookie" {
            return;
        }
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        if object.name.as_str() != "document" {
            return;
        }
        self.report(RULE_NAME, "cookies", it.span);
    }

    /// Shapes (b) and (c): flags an ExpressJS `res.cookie(name, value)` write or
    /// a NodeJS `res.setHeader('Set-Cookie', …)` write.
    pub(crate) fn check_cookies_call(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        match member.property.name.as_str() {
            // (b) `.cookie(name, value)` with at least one argument.
            "cookie" => {
                if !it.arguments.is_empty() {
                    self.report(RULE_NAME, "cookies", it.span);
                }
            }
            // (c) `.setHeader('Set-Cookie', …)`.
            "setHeader" => {
                let is_set_cookie = matches!(
                    it.arguments.first(),
                    Some(Argument::StringLiteral(name))
                        if name.value.as_str().eq_ignore_ascii_case("Set-Cookie")
                );
                if is_set_cookie {
                    self.report(RULE_NAME, "cookies", it.span);
                }
            }
            _ => {}
        }
    }
}
