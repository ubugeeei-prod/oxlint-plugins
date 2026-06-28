//! Rule `x-powered-by` (SonarJS key S5689).
//!
//! Clean-room port from the public RSPEC S5689 description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Disclosing the technology stack of a web application (its "fingerprint")
//! through response headers helps attackers target known vulnerabilities of the
//! detected components. Express.js advertises itself by sending the
//! `X-Powered-By` HTTP header on every response. The recommended hardening is to
//! remove that header, e.g. `app.disable("x-powered-by")` or the helmet
//! `hidePoweredBy` middleware.
//!
//! ## Zero-FP subset
//!
//! Faithfully reproducing S5689 in full means flagging an `express()`
//! application whose `X-Powered-By` header is never disabled anywhere in the
//! module — a whole-program absence check that cannot be decided without cross-
//! file dataflow and therefore cannot be made false-positive-free here. Instead
//! this port flags the unambiguous *opposite* action: source that explicitly
//! (re-)enables the header. A `CallExpression` is reported when its callee is a
//! static member expression and either:
//! - the method is `enable` and the first argument is the string literal
//!   `x-powered-by` (case-insensitive), as in `app.enable("x-powered-by")`; or
//! - the method is `set` and the arguments are the string literal
//!   `x-powered-by` (case-insensitive) followed by the boolean literal `true`,
//!   as in `app.set("x-powered-by", true)`.
//!
//! Both forms switch the fingerprinting header on deliberately, so the match is
//! effectively zero-false-positive. The distinctive header name keeps unrelated
//! `enable`/`set` calls from matching. The call span is reported. Under-
//! reporting (we do not flag the mere absence of a `disable` call) is preferred
//! over the false positives a presence heuristic would introduce.
//!
//! ## Flagged
//! ```js
//! app.enable("x-powered-by");      // header turned on explicitly
//! app.set("x-powered-by", true);   // header turned on explicitly
//! ```
//!
//! ## Not Flagged
//! ```js
//! app.disable("x-powered-by");     // recommended hardening
//! app.set("x-powered-by", false);  // header turned off
//! app.enable("etag");              // unrelated setting
//! const app = express();           // absence of disable not flagged here
//! ```

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "x-powered-by";

/// Returns the value of `arg` when it is a string literal, else `None`.
fn string_arg<'a>(arg: &'a Argument<'a>) -> Option<&'a str> {
    match arg {
        Argument::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_x_powered_by(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        let Some(first) = expr.arguments.first() else {
            return;
        };
        let Some(header) = string_arg(first) else {
            return;
        };
        if !header.eq_ignore_ascii_case("x-powered-by") {
            return;
        }
        let flagged = match member.property.name.as_str() {
            // `app.enable("x-powered-by")` switches the header on.
            "enable" => true,
            // `app.set("x-powered-by", true)` switches the header on.
            "set" => matches!(
                expr.arguments.get(1),
                Some(Argument::BooleanLiteral(b)) if b.value
            ),
            _ => false,
        };
        if flagged {
            self.report(RULE_NAME, "xPoweredBy", expr.span);
        }
    }
}
