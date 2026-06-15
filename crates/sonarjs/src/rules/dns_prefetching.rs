//! Rule `dns-prefetching` (SonarJS key S5743).
//!
//! Clean-room port from the public RSPEC S5743 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! By default browsers perform DNS prefetching to lower the latency of links a
//! page offers, but doing so without the user's consent leaks which sites the
//! user is about to visit to network eavesdroppers. The helmet middleware can
//! re-enable prefetching by emitting an `X-DNS-Prefetch-Control: on` header,
//! which it does when `dnsPrefetchControl` is configured with `{ allow: true }`.
//!
//! ## Zero-FP subset
//!
//! This port flags a `CallExpression` whose callee (after unwrapping
//! parentheses/`as`/etc. via `get_inner_expression`) is a static member
//! expression named `dnsPrefetchControl`, whose first argument is an object
//! literal containing a property whose key (static identifier or string
//! literal) is exactly `allow` set to the boolean literal `true`. The
//! `dnsPrefetchControl` method name is distinctive to helmet, so flagging only
//! the explicit `{ allow: true }` opt-in is effectively zero-false-positive in
//! practice. The call span is reported.
//!
//! The match keys off the property name only, so `foo.dnsPrefetchControl({
//! allow: true })` on an unrelated receiver is also flagged; the distinctive
//! method name keeps this effectively zero-false-positive while catching
//! `helmet.dnsPrefetchControl(...)` regardless of how `helmet` was imported.
//!
//! ## Flagged
//! ```js
//! helmet.dnsPrefetchControl({ allow: true });   // re-enables DNS prefetching
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet.dnsPrefetchControl({ allow: false });  // prefetching stays disabled
//! helmet.dnsPrefetchControl();                   // no config, default applies
//! helmet.dnsPrefetchControl({ allow: x });       // non-literal value
//! bar();                                         // unrelated callee
//! ```

use oxc_ast::ast::{Argument, CallExpression, Expression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "dns-prefetching";

impl Scanner<'_> {
    pub(crate) fn check_dns_prefetching(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        if member.property.name != "dnsPrefetchControl" {
            return;
        }
        let Some(Argument::ObjectExpression(obj)) = expr.arguments.first() else {
            return;
        };
        let allows = obj.properties.iter().any(|prop| {
            let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                return false;
            };
            let key = match &prop.key {
                PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                _ => return false,
            };
            key == "allow" && matches!(&prop.value, Expression::BooleanLiteral(b) if b.value)
        });
        if allows {
            self.report(RULE_NAME, "dnsPrefetching", expr.span);
        }
    }
}
