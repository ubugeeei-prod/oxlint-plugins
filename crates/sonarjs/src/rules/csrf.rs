//! Rule `csrf` (SonarJS key S4502).
//!
//! Clean-room port from public RSPEC S4502 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Cross-site request forgery (CSRF) attacks trick an authenticated user into
//! performing unintended state-changing actions. The `csurf` middleware
//! protects against this, but its `ignoreMethods` option lists HTTP methods
//! for which the CSRF token is *not* checked. Listing an unsafe,
//! state-changing verb (`POST`, `PUT`, `DELETE`, `PATCH`) there silently
//! disables the protection for exactly the requests that need it.
//!
//! ## Zero-FP subset
//!
//! This port flags a `CallExpression` whose callee (after unwrapping
//! parentheses/non-null assertions) is an `Identifier` named `csrf` — the
//! `csurf` middleware factory — whose first argument is an object literal with
//! an `ignoreMethods` property whose value is an array literal containing at
//! least one string literal naming an unsafe verb (`POST`/`PUT`/`DELETE`/
//! `PATCH`, compared case-insensitively). The `csrf(...)` callee combined with
//! the `ignoreMethods` key is distinctive to `csurf` configuration, so this is
//! effectively zero-false-positive. The call expression span is reported.
//!
//! ## Flagged
//! ```js
//! csrf({ ignoreMethods: ["POST", "GET"] }); // POST is state-changing
//! csrf({ ignoreMethods: ["PUT"] });         // PUT is state-changing
//! ```
//!
//! ## Not Flagged
//! ```js
//! csrf({ ignoreMethods: ["GET", "HEAD", "OPTIONS"] }); // only safe verbs
//! csrf({ cookie: true });   // no ignoreMethods key
//! csrf();                   // no configuration object
//! foo({ ignoreMethods: ["POST"] }); // not the csrf factory
//! ```

use oxc_ast::ast::{ArrayExpressionElement, CallExpression, Expression, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "csrf";

/// Unsafe, state-changing HTTP verbs whose presence in `ignoreMethods`
/// disables CSRF protection. Compared case-insensitively.
const UNSAFE_METHODS: [&str; 4] = ["POST", "PUT", "DELETE", "PATCH"];

impl Scanner<'_> {
    pub(crate) fn check_csrf(&mut self, expr: &CallExpression<'_>) {
        let Expression::Identifier(ident) = expr.callee.get_inner_expression() else {
            return;
        };
        if ident.name != "csrf" {
            return;
        }
        let Some(first) = expr.arguments.first().and_then(|arg| arg.as_expression()) else {
            return;
        };
        let Expression::ObjectExpression(obj) = first.get_inner_expression() else {
            return;
        };
        for property in &obj.properties {
            if let Some(prop) = property.as_property() {
                let key = match &prop.key {
                    PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                    PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                    _ => continue,
                };
                if key != "ignoreMethods" {
                    continue;
                }
                if ignore_methods_disables_protection(&prop.value) {
                    self.report(RULE_NAME, "csrf", expr.span);
                }
                return;
            }
        }
    }
}

/// Returns `true` when an `ignoreMethods` value is an array literal listing at
/// least one unsafe, state-changing HTTP verb.
fn ignore_methods_disables_protection(value: &Expression<'_>) -> bool {
    let Expression::ArrayExpression(array) = value.get_inner_expression() else {
        return false;
    };
    array_has_unsafe_method(&array.elements)
}

fn array_has_unsafe_method(elements: &[ArrayExpressionElement<'_>]) -> bool {
    elements.iter().any(|element| {
        if let ArrayExpressionElement::StringLiteral(lit) = element {
            UNSAFE_METHODS
                .iter()
                .any(|verb| lit.value.eq_ignore_ascii_case(verb))
        } else {
            false
        }
    })
}
