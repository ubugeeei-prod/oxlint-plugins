//! Rule `no-weak-cipher` (SonarJS key S5547).
//!
//! Clean-room port. Reports direct use of weak cipher algorithm names in
//! Node-style crypto cipher/decipher factory calls. This implementation is
//! intentionally conservative about dataflow: only a string-literal first
//! argument is inspected, and dynamic algorithm values are skipped.
//!
//! **Flagged**:
//! - `crypto.createCipheriv("des-cbc", key, iv)`
//! - `createCipher("rc4", password)`
//! - `crypto.createDecipheriv("bf-cbc", key, iv)`
//!
//! **Not flagged**:
//! - `crypto.createCipheriv("aes-256-gcm", key, iv)`
//! - `crypto.createCipheriv(algorithm, key, iv)` — dynamic algorithm.
//! - `cipher.create("des-cbc")` — different API name.
//!
//! Behaviour is reproduced from public RSPEC S5547 documentation only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-weak-cipher";

fn first_string_argument<'a>(args: &[Argument<'a>]) -> Option<&'a str> {
    let first = args.first()?;
    match first {
        Argument::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

fn normalized_byte(byte: u8) -> u8 {
    if byte == b'_' {
        b'-'
    } else {
        byte.to_ascii_lowercase()
    }
}

fn has_algorithm_prefix(value: &str, prefix: &str) -> bool {
    let value = value.trim().as_bytes();
    let prefix = prefix.as_bytes();
    if value.len() < prefix.len() {
        return false;
    }
    for (index, prefix_byte) in prefix.iter().enumerate() {
        if normalized_byte(value[index]) != *prefix_byte {
            return false;
        }
    }
    value.len() == prefix.len() || matches!(normalized_byte(value[prefix.len()]), b'-' | b'/')
}

fn is_weak_cipher_algorithm(value: &str) -> bool {
    ["des", "rc2", "rc4", "bf", "blowfish", "idea"]
        .iter()
        .any(|prefix| has_algorithm_prefix(value, prefix))
}

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

impl Scanner<'_> {
    pub(crate) fn check_no_weak_cipher(&mut self, it: &CallExpression<'_>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let Some(algorithm) = first_string_argument(&it.arguments) else {
            return;
        };
        if is_weak_cipher_algorithm(algorithm) && callee_is_cipher_factory(&it.callee) {
            self.report(RULE_NAME, "weakCipher", it.span);
        }
    }
}
