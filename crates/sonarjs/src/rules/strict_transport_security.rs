//! Rule `strict-transport-security` (SonarJS key S5739).
//!
//! Clean-room port from the public RSPEC S5739 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! The HTTP Strict-Transport-Security (HSTS) header instructs browsers to only
//! reach a site over HTTPS, defending against protocol-downgrade and cookie-
//! hijacking attacks. The helmet middleware emits this header when its `hsts`
//! method is configured. A policy is weak when it does not cover subdomains
//! (`includeSubDomains: false`) or when its lifetime is too short. Per the
//! RSPEC the recommended minimum `maxAge` is six months, i.e. `15552000`
//! seconds (one year, `31536000`, is preferred); anything shorter is flagged.
//!
//! ## Zero-FP subset
//!
//! This port flags a `CallExpression` whose callee (after unwrapping
//! parentheses/`as`/etc. via `get_inner_expression`) is a static member
//! expression named `hsts`, whose first argument is an object literal
//! containing EITHER:
//! - an `includeSubDomains` property set to the boolean literal `false`; OR
//! - a `maxAge` property set to a numeric literal strictly below `15552000`.
//!
//! The match keys off the property name only, so `foo.hsts({ ... })` on an
//! unrelated receiver is also flagged; the distinctive `hsts` method name keeps
//! this effectively zero-false-positive while catching `helmet.hsts(...)`
//! regardless of how `helmet` was imported. Non-literal values are never
//! flagged. The call span is reported.
//!
//! ## Flagged
//! ```js
//! helmet.hsts({ includeSubDomains: false });   // subdomains unprotected
//! helmet.hsts({ maxAge: 3153600 });            // lifetime under six months
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet.hsts({ includeSubDomains: true, maxAge: 31536000 }); // strong policy
//! helmet.hsts({ maxAge: 15552000 });           // exactly the minimum
//! helmet.hsts({ maxAge: x });                  // non-literal value
//! bar();                                       // unrelated callee
//! ```

use oxc_ast::ast::{Argument, CallExpression, Expression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "strict-transport-security";

/// Minimum recommended HSTS `max-age` in seconds (six months) per RSPEC S5739.
const MIN_MAX_AGE: f64 = 15_552_000.0;

impl Scanner<'_> {
    pub(crate) fn check_strict_transport_security(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        if member.property.name != "hsts" {
            return;
        }
        let Some(Argument::ObjectExpression(obj)) = expr.arguments.first() else {
            return;
        };
        let weak = obj.properties.iter().any(|prop| {
            let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                return false;
            };
            let key = match &prop.key {
                PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                _ => return false,
            };
            match key {
                "includeSubDomains" => {
                    matches!(&prop.value, Expression::BooleanLiteral(b) if !b.value)
                }
                "maxAge" => {
                    matches!(&prop.value, Expression::NumericLiteral(n) if n.value < MIN_MAX_AGE)
                }
                _ => false,
            }
        });
        if weak {
            self.report(RULE_NAME, "strictTransportSecurity", expr.span);
        }
    }
}
