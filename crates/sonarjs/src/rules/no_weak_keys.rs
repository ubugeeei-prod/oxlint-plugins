//! Rule `no-weak-keys` (SonarJS key S4426).
//!
//! Clean-room port from the public RSPEC S4426 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Generating an asymmetric cryptographic key with too small a size leaves the
//! ciphertext recoverable by an attacker. Per the public RSPEC guidance, RSA,
//! DSA and Diffie-Hellman keys must use a modulus of at least 2048 bits, and
//! elliptic-curve keys must use a curve of at least 224 bits. Node's
//! `crypto.generateKeyPair` / `crypto.generateKeyPairSync` accept these
//! parameters through their options object as `modulusLength` (a number) and
//! `namedCurve` (a string).
//!
//! ## Zero-FP subset
//!
//! This port flags a `CallExpression` whose callee (after unwrapping via
//! `get_inner_expression`) is a static member expression named
//! `generateKeyPair` or `generateKeyPairSync`, and one of whose arguments is an
//! object literal containing EITHER:
//! - a `modulusLength` property whose value is a numeric literal below 2048; OR
//! - a `namedCurve` property whose value is a string literal naming a curve
//!   known to provide fewer than 224 bits of strength.
//!
//! The distinctive `generateKeyPair*` method name combined with an explicit
//! literal weak parameter makes this effectively zero-false-positive. Only
//! literal values are inspected; variables and computed values are never
//! guessed. The call span is reported.
//!
//! ### Weak thresholds / curve set used
//!
//! - `modulusLength` (RSA/DSA/DH): a numeric literal strictly less than `2048`.
//! - `namedCurve` (EC): the set of named curves below the 224-bit floor, drawn
//!   from the SECG / X9.62 / Brainpool curve names Node's crypto recognises
//!   (`secp112r1`, `secp112r2`, `secp128r1`, `secp128r2`, `secp160k1`,
//!   `secp160r1`, `secp160r2`, `secp192k1`, `prime192v1`, `prime192v2`,
//!   `prime192v3`, `brainpoolP160r1`, `brainpoolP160t1`, `brainpoolP192r1`,
//!   `brainpoolP192t1`). Curves at or above 224 bits (e.g. `secp224k1`,
//!   `secp256r1`, `prime256v1`, `secp384r1`, `secp521r1`) are not flagged.
//!
//! ## Flagged
//! ```js
//! crypto.generateKeyPairSync('rsa', { modulusLength: 1024 }); // weak RSA
//! crypto.generateKeyPair('ec', { namedCurve: 'secp112r2' }, cb); // weak curve
//! ```
//!
//! ## Not Flagged
//! ```js
//! crypto.generateKeyPairSync('rsa', { modulusLength: 2048 }); // strong
//! crypto.generateKeyPairSync('rsa', { modulusLength: x });    // non-literal
//! foo({ modulusLength: 1024 });                               // wrong callee
//! ```

use oxc_ast::ast::{Argument, CallExpression, Expression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-weak-keys";

/// Minimum acceptable RSA/DSA/DH modulus length, in bits.
const MIN_MODULUS_LENGTH: f64 = 2048.0;

/// Named elliptic curves providing fewer than 224 bits of strength.
const WEAK_CURVES: [&str; 15] = [
    "secp112r1",
    "secp112r2",
    "secp128r1",
    "secp128r2",
    "secp160k1",
    "secp160r1",
    "secp160r2",
    "secp192k1",
    "prime192v1",
    "prime192v2",
    "prime192v3",
    "brainpoolP160r1",
    "brainpoolP160t1",
    "brainpoolP192r1",
    "brainpoolP192t1",
];

impl Scanner<'_> {
    pub(crate) fn check_no_weak_keys(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        let name = member.property.name.as_str();
        if name != "generateKeyPair" && name != "generateKeyPairSync" {
            return;
        }
        let weak = expr.arguments.iter().any(|arg| {
            let Argument::ObjectExpression(obj) = arg else {
                return false;
            };
            obj.properties.iter().any(|prop| {
                let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                    return false;
                };
                let key = match &prop.key {
                    PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                    PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                    _ => return false,
                };
                match key {
                    "modulusLength" => {
                        matches!(&prop.value, Expression::NumericLiteral(lit) if lit.value < MIN_MODULUS_LENGTH)
                    }
                    "namedCurve" => {
                        matches!(&prop.value, Expression::StringLiteral(lit) if WEAK_CURVES.contains(&lit.value.as_str()))
                    }
                    _ => false,
                }
            })
        });
        if weak {
            self.report(RULE_NAME, "weakKeys", expr.span);
        }
    }
}
