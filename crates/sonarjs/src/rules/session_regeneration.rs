//! Rule `session-regeneration` (SonarJS key S5876).
//!
//! Clean-room port from the public RSPEC S5876 description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! When a user authenticates, the existing (pre-authentication) session must be
//! discarded and a brand-new session created. Re-using the same session id
//! across the authentication boundary leaves the application open to *session
//! fixation*: an attacker who can plant a known session id before login then
//! shares the now-authenticated session afterwards. With Passport.js the fix is
//! to call `req.session.regenerate(...)` inside the route handler that runs
//! after authentication succeeds.
//!
//! ## Zero-FP subset
//!
//! This port targets the canonical Express + Passport route shape, where the
//! authentication middleware and the post-authentication handler are arguments
//! of the same route-registration call:
//!
//! ```js
//! app.post('/login',
//!   passport.authenticate('local', { failureRedirect: '/login' }),
//!   function (req, res) {        // runs only after auth succeeds
//!     res.redirect('/');
//!   });
//! ```
//!
//! The check fires on a `CallExpression` that has BOTH:
//!   * an argument that is itself a call to `passport.authenticate(...)`
//!     (object identifier `passport`, property `authenticate`), and
//!   * an inline function or arrow-function argument (the route handler).
//!
//! Each such inline handler whose source text does not mention `regenerate`
//! (i.e. never calls `req.session.regenerate(...)`) is reported. Requiring the
//! literal `passport.authenticate` callee plus a sibling inline handler makes
//! the pattern distinctive enough to be effectively false-positive-free.
//!
//! ## Deliberate narrowing (documented follow-ups)
//!   * Handlers passed by name (`app.post('/login', passport.authenticate(...),
//!     onLogin)`) are NOT flagged — their body is not available at this node, so
//!     we cannot verify regeneration and choose to under-report.
//!   * Aliased passport objects (`const p = passport; p.authenticate(...)`) and
//!     the custom-verify-callback form (`passport.authenticate('local',
//!     (req,res) => {...})`) are out of scope.
//!   * Presence of the substring `regenerate` anywhere in the handler is treated
//!     as compliant; this favours false negatives over false positives.

use oxc_ast::ast::{CallExpression, Expression};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "session-regeneration";

impl Scanner<'_> {
    pub(crate) fn check_session_regeneration(&mut self, call: &CallExpression<'_>) {
        let has_passport_authenticate = call
            .arguments
            .iter()
            .filter_map(|arg| arg.as_expression())
            .any(is_passport_authenticate);
        if !has_passport_authenticate {
            return;
        }

        let mut spans: SmallVec<[Span; 2]> = SmallVec::new();
        for arg in &call.arguments {
            let Some(expr) = arg.as_expression() else {
                continue;
            };
            let Some(span) = handler_span(expr) else {
                continue;
            };
            if !self.text(span).contains("regenerate") {
                spans.push(span);
            }
        }

        for span in spans {
            self.report(RULE_NAME, "createSession", span);
        }
    }
}

/// True when `expr` is a call to `passport.authenticate(...)`.
fn is_passport_authenticate(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(inner) = expr.get_inner_expression() else {
        return false;
    };
    let Expression::StaticMemberExpression(member) = inner.callee.get_inner_expression() else {
        return false;
    };
    if member.property.name != "authenticate" {
        return false;
    }
    matches!(
        member.object.get_inner_expression(),
        Expression::Identifier(id) if id.name == "passport"
    )
}

/// Span of an inline function/arrow handler argument, or `None` for any other
/// argument kind (string, options object, named reference, ...).
fn handler_span(expr: &Expression<'_>) -> Option<Span> {
    match expr.get_inner_expression() {
        Expression::FunctionExpression(func) => Some(func.span),
        Expression::ArrowFunctionExpression(func) => Some(func.span),
        _ => None,
    }
}
