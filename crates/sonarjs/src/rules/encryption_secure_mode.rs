//! Rule `encryption-secure-mode` (SonarJS key S5542).
//!
//! Clean-room port. Block ciphers must be used with a secure mode of
//! operation. The ECB mode encrypts identical plaintext blocks to identical
//! ciphertext blocks, leaking structure, and the CBC mode on its own provides
//! no message authentication, leaving the ciphertext open to manipulation.
//! Authenticated modes such as GCM or CCM should be used instead.
//!
//! This implements only an unambiguous, zero-false-positive subset that keys
//! off Node's `crypto` cipher/decipher factory calls whose first argument is a
//! string-literal cipher specification naming an insecure mode segment.
//!
//! **Flagged** — a `CallExpression` whose callee (after unwrapping
//! parentheses) is an identifier or member named `createCipheriv`,
//! `createDecipheriv`, `createCipher`, or `createDecipher`, and whose first
//! argument is a string literal whose lower-cased value contains the mode
//! segment `-ecb` or `-cbc`:
//! - `crypto.createCipheriv("aes-128-cbc", key, iv)` — CBC mode.
//! - `createCipheriv("AES-256-ECB", key, iv)` — ECB mode (case-insensitive).
//! - `crypto.createDecipheriv("des-ecb", key, iv)`.
//!
//! **Not flagged**:
//! - `crypto.createCipheriv("aes-256-gcm", key, iv)` — secure authenticated
//!   mode (GCM/CCM/CTR/CFB/OFB are not reported).
//! - `crypto.createCipheriv(algorithm, key, iv)` — dynamic, non-literal first
//!   argument.
//! - `foo("aes-128-cbc")` — callee is not a crypto cipher factory.
//!
//! Behaviour is reproduced from public RSPEC S5542 documentation only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "encryption-secure-mode";

fn is_cipher_factory_name(name: &str) -> bool {
    matches!(
        name,
        "createCipher" | "createCipheriv" | "createDecipher" | "createDecipheriv"
    )
}

fn callee_is_cipher_factory(callee: &Expression<'_>) -> bool {
    match callee.get_inner_expression() {
        Expression::Identifier(ident) => is_cipher_factory_name(ident.name.as_str()),
        Expression::StaticMemberExpression(member) => {
            is_cipher_factory_name(member.property.name.as_str())
        }
        _ => false,
    }
}

fn first_string_argument<'a>(args: &[Argument<'a>]) -> Option<&'a str> {
    match args.first()? {
        Argument::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

fn names_insecure_mode(spec: &str) -> bool {
    let lowered = spec.to_ascii_lowercase();
    lowered.contains("-cbc") || lowered.contains("-ecb")
}

impl Scanner<'_> {
    pub(crate) fn check_encryption_secure_mode(&mut self, it: &CallExpression<'_>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        if !callee_is_cipher_factory(&it.callee) {
            return;
        }
        let Some(spec) = first_string_argument(&it.arguments) else {
            return;
        };
        if names_insecure_mode(spec) {
            self.report(RULE_NAME, "insecureCipherMode", it.span);
        }
    }
}
