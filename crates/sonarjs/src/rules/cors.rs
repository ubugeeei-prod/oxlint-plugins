//! Rule `cors` (SonarJS key S5122).
//!
//! Clean-room port from public RSPEC S5122 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Cross-Origin Resource Sharing (CORS) lets a server relax the browser's
//! same-origin policy. Configuring it to trust *any* origin — the wildcard
//! `"*"` — effectively disables access control: every website can read the
//! responses, which has caused real vulnerabilities (e.g. CVE-2018-0269,
//! CVE-2017-14460). The fix is to allow only specific, trusted origins.
//!
//! ## Zero-FP subset
//!
//! Only the unambiguous wildcard shapes are flagged. The `Access-Control-
//! Allow-Origin` header name and the `cors({ origin })` middleware option are
//! distinctive enough that matching them syntactically is effectively
//! zero-false-positive. Three forms are reported (the call expression span):
//!
//! (a) `res.setHeader("Access-Control-Allow-Origin", "*")` — a `.setHeader`
//!     member call with exactly two string-literal arguments where the first
//!     (compared case-insensitively, as HTTP header names are case-insensitive)
//!     is `Access-Control-Allow-Origin` and the second is `"*"`.
//! (b) `cors({ origin: "*" })` — a call to the bare identifier `cors` (the
//!     `cors` Express middleware factory) whose first argument is an object
//!     literal with an `origin` property whose value is the string `"*"`.
//! (c) `res.writeHead(200, { "Access-Control-Allow-Origin": "*" })` — any call
//!     with an object-literal argument that carries an `Access-Control-Allow-
//!     Origin` property (case-insensitive key) set to `"*"`. This covers the
//!     headers-object form shown for the Node.js `http` module.
//!
//! ## Flagged
//! ```js
//! res.setHeader("Access-Control-Allow-Origin", "*");
//! cors({ origin: "*" });
//! res.writeHead(200, { "Access-Control-Allow-Origin": "*" });
//! ```
//!
//! ## Not flagged
//! ```js
//! res.setHeader("Access-Control-Allow-Origin", "https://example.com"); // specific origin
//! res.setHeader("Access-Control-Allow-Origin", origin);                // dynamic — may be validated
//! cors({ origin: "https://example.com" });                            // specific origin
//! cors();                                                              // no configuration object
//! res.setHeader("Content-Type", "text/plain");                        // unrelated header
//! ```

use oxc_ast::ast::{CallExpression, Expression, ObjectExpression, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "cors";

/// Canonical wildcard CORS origin value.
const WILDCARD: &str = "*";
/// The header that controls which origins may read a CORS response.
const ALLOW_ORIGIN_HEADER: &str = "Access-Control-Allow-Origin";

impl Scanner<'_> {
    pub(crate) fn check_cors(&mut self, expr: &CallExpression<'_>) {
        if call_is_permissive_cors(expr) {
            self.report(RULE_NAME, "cors", expr.span);
        }
    }
}

/// Returns `true` for any of the three permissive-CORS shapes:
/// (a) `res.setHeader("Access-Control-Allow-Origin", "*")`,
/// (b) `cors({ origin: "*" })`, or
/// (c) a headers object literal with `Access-Control-Allow-Origin` set to `"*"`
///     (e.g. `res.writeHead(200, { ... })`).
fn call_is_permissive_cors(expr: &CallExpression<'_>) -> bool {
    match expr.callee.get_inner_expression() {
        Expression::StaticMemberExpression(member)
            if member.property.name == "setHeader" && set_header_is_wildcard(expr) =>
        {
            true
        }
        Expression::Identifier(ident) if ident.name == "cors" && cors_origin_is_wildcard(expr) => {
            true
        }
        _ => any_argument_sets_wildcard_header(expr),
    }
}

/// Returns `true` for `setHeader("Access-Control-Allow-Origin", "*")`: exactly
/// two string-literal arguments, the header name (case-insensitive) and `"*"`.
fn set_header_is_wildcard(expr: &CallExpression<'_>) -> bool {
    if expr.arguments.len() != 2 {
        return false;
    }
    let Some(name) = string_literal_value(expr, 0) else {
        return false;
    };
    if !name.eq_ignore_ascii_case(ALLOW_ORIGIN_HEADER) {
        return false;
    }
    matches!(string_literal_value(expr, 1), Some(value) if value == WILDCARD)
}

/// Returns the string-literal value of the argument at `index`, if it is one.
fn string_literal_value<'a>(expr: &'a CallExpression<'a>, index: usize) -> Option<&'a str> {
    let arg = expr.arguments.get(index)?.as_expression()?;
    match arg.get_inner_expression() {
        Expression::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

/// Returns `true` for `cors({ origin: "*" })`: the first argument is an object
/// literal whose `origin` property is the wildcard string.
fn cors_origin_is_wildcard(expr: &CallExpression<'_>) -> bool {
    let Some(first) = expr.arguments.first().and_then(|arg| arg.as_expression()) else {
        return false;
    };
    let Expression::ObjectExpression(obj) = first.get_inner_expression() else {
        return false;
    };
    object_property_is_wildcard(obj, |key| key == "origin")
}

/// Returns `true` when any argument is an object literal carrying an
/// `Access-Control-Allow-Origin` property (case-insensitive) set to `"*"`.
fn any_argument_sets_wildcard_header(expr: &CallExpression<'_>) -> bool {
    expr.arguments.iter().any(|arg| match arg.as_expression() {
        Some(expression) => match expression.get_inner_expression() {
            Expression::ObjectExpression(obj) => object_property_is_wildcard(obj, |key| {
                key.eq_ignore_ascii_case(ALLOW_ORIGIN_HEADER)
            }),
            _ => false,
        },
        None => false,
    })
}

/// Returns `true` when `obj` has a (identifier or string) property key matching
/// `key_matches` whose value is the string literal `"*"`.
fn object_property_is_wildcard(
    obj: &ObjectExpression<'_>,
    key_matches: impl Fn(&str) -> bool,
) -> bool {
    for property in &obj.properties {
        let Some(prop) = property.as_property() else {
            continue;
        };
        let key = match &prop.key {
            PropertyKey::StaticIdentifier(id) => id.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => continue,
        };
        if !key_matches(key) {
            continue;
        }
        return matches!(
            prop.value.get_inner_expression(),
            Expression::StringLiteral(lit) if lit.value == WILDCARD
        );
    }
    false
}
