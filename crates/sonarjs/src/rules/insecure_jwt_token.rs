//! Rule `insecure-jwt-token` (SonarJS key S5659).
//!
//! Clean-room port from public RSPEC S5659 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! A JSON Web Token signed or verified with the `none` algorithm carries no
//! signature, so its payload can be forged freely — an attacker can mint a
//! token impersonating any user. JWTs must use a strong signing algorithm
//! (e.g. `HS256`, `RS256`) and the verifier must reject the `none` algorithm.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` (the options object passed to
//! `jwt.sign` / `jwt.verify`) when either:
//!
//! * its key (a static identifier or string literal) is exactly `algorithm`
//!   and its value is a string literal equal to `none` (case-insensitive); or
//! * its key is exactly `algorithms` and its value is an array literal that
//!   contains a string literal equal to `none` (case-insensitive).
//!
//! The `algorithm` / `algorithms` option keys combined with the literal `none`
//! algorithm name are distinctive to JWT configuration, so flagging only these
//! shapes is effectively zero-false-positive. The property span is reported.
//!
//! ## Flagged
//! ```js
//! jwt.sign(payload, key, { algorithm: 'none' });
//! jwt.verify(token, key, { algorithms: ['none'] });
//! const opts = { algorithm: 'NONE' }; // case-insensitive
//! ```
//!
//! ## Not Flagged
//! ```js
//! jwt.sign(payload, key, { algorithm: 'HS256' });    // strong algorithm
//! jwt.verify(token, key, { algorithms: ['RS256'] }); // strong algorithm
//! const opts = { algorithm: x };                     // non-literal value
//! const opts = { other: 'none' };                    // different key
//! ```

use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "insecure-jwt-token";

impl Scanner<'_> {
    pub(crate) fn check_insecure_jwt_token_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        let uses_none = match key {
            "algorithm" => is_none_string(&it.value),
            "algorithms" => value_array_has_none(&it.value),
            _ => return,
        };
        if uses_none {
            self.report(RULE_NAME, "insecureJwtToken", it.span);
        }
    }
}

/// Returns `true` when an expression is a string literal equal to `none`
/// (compared case-insensitively).
fn is_none_string(value: &Expression<'_>) -> bool {
    matches!(value, Expression::StringLiteral(lit) if lit.value.eq_ignore_ascii_case("none"))
}

/// Returns `true` when an expression is an array literal containing at least
/// one string literal equal to `none` (compared case-insensitively).
fn value_array_has_none(value: &Expression<'_>) -> bool {
    let Expression::ArrayExpression(array) = value else {
        return false;
    };
    array.elements.iter().any(|element| {
        if let ArrayExpressionElement::StringLiteral(lit) = element {
            lit.value.eq_ignore_ascii_case("none")
        } else {
            false
        }
    })
}
