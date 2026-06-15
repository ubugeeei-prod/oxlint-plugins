//! Rule `hashing` (SonarJS key S4790).
//!
//! Clean-room port. Flags calls that clearly request obsolete hashing
//! algorithms in common JavaScript crypto APIs. The implementation is
//! intentionally syntactic: it reports only when the algorithm name is a string
//! literal passed directly to `createHash(...)` or a WebCrypto
//! `*.subtle.digest(...)` call.
//!
//! **Flagged**:
//! - `crypto.createHash("md5")`
//! - `createHash("SHA-1")`
//! - `crypto.subtle.digest("SHA-1", data)`
//!
//! **Not flagged**:
//! - `crypto.createHash("sha256")`
//! - `crypto.createHash(algorithm)` — dynamic algorithm.
//! - `digest("SHA-1", data)` — bare function call with no WebCrypto shape.
//!
//! Behaviour is reproduced from public RSPEC S4790 documentation only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "hashing";

fn first_string_argument<'a>(args: &[Argument<'a>]) -> Option<&'a str> {
    let first = args.first()?;
    match first {
        Argument::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

fn normalized_algorithm_equals(value: &str, expected: &[u8]) -> bool {
    let mut expected_index = 0usize;
    for byte in value.bytes() {
        if byte == b'-' || byte == b'_' || byte.is_ascii_whitespace() {
            continue;
        }
        if expected_index >= expected.len() || byte.to_ascii_lowercase() != expected[expected_index]
        {
            return false;
        }
        expected_index += 1;
    }
    expected_index == expected.len()
}

fn is_weak_hash_algorithm(value: &str) -> bool {
    normalized_algorithm_equals(value, b"md4")
        || normalized_algorithm_equals(value, b"md5")
        || normalized_algorithm_equals(value, b"sha1")
}

fn callee_is_create_hash(callee: &Expression<'_>) -> bool {
    match callee.get_inner_expression() {
        Expression::Identifier(ident) => ident.name == "createHash",
        Expression::StaticMemberExpression(member) => member.property.name == "createHash",
        _ => false,
    }
}

fn callee_is_subtle_digest(callee: &Expression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = callee.get_inner_expression() else {
        return false;
    };
    if member.property.name != "digest" {
        return false;
    }
    match member.object.get_inner_expression() {
        Expression::Identifier(ident) => ident.name == "subtle",
        Expression::StaticMemberExpression(object_member) => {
            object_member.property.name == "subtle"
        }
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_hashing(&mut self, it: &CallExpression<'_>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let Some(algorithm) = first_string_argument(&it.arguments) else {
            return;
        };
        if !is_weak_hash_algorithm(algorithm) {
            return;
        }
        let callee = it.callee.get_inner_expression();
        if callee_is_create_hash(callee) || callee_is_subtle_digest(callee) {
            self.report(RULE_NAME, "weakHash", it.span);
        }
    }
}
