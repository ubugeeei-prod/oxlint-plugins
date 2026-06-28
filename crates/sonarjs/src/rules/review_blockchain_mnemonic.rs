//! Rule `review-blockchain-mnemonic` (SonarJS key S7639).
//!
//! Clean-room port from the public RSPEC S7639 description and the public
//! BIP-39 specification only; no upstream SonarJS source, tests, fixtures, or
//! message strings were consulted or copied. Structure mirrors the sibling
//! secret-detection rule `no-hardcoded-secrets`.
//!
//! A blockchain wallet "mnemonic" (also called a seed phrase or recovery
//! phrase, per BIP-39) is the human-readable backup of a wallet's private key.
//! Anyone who learns the phrase gains full control of the wallet's funds.
//! Hardcoding such a phrase in source code discloses it and must be reviewed.
//!
//! ## Narrow form
//!
//! To stay false-positive-free without a TypeScript type checker, dataflow, or
//! a full BIP-39 dictionary, this port flags a string literal only when BOTH
//! signals agree:
//!
//!  1. **Target name** — the literal is bound/assigned to an identifier or
//!     object-property key whose whole name (case-insensitively, after
//!     stripping `_`) is a mnemonic word:
//!     `mnemonic`, `mnemonics`, `seedphrase`, `recoveryphrase`, `secretphrase`,
//!     `backupphrase`, `walletmnemonic`, `walletphrase`.
//!  2. **BIP-39 shape** — the string value splits on ASCII whitespace into
//!     exactly 12, 15, 18, 21, or 24 tokens, and every token is 3–8 lowercase
//!     ASCII letters (`a`–`z`). These are exactly the structural constraints of
//!     a BIP-39 mnemonic, so a real seed phrase always passes while ordinary
//!     prose assigned to such a variable does not.
//!
//! Requiring both signals means an embedded 2048-word dictionary is not needed
//! and there are effectively no false positives.
//!
//! ## Flagged
//! ```js
//! const mnemonic = "legal winner thank year wave sausage worth useful legal winner thank yellow";
//! const wallet = { seedPhrase: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about" };
//! recoveryPhrase = "uncle scare brave coyote leaf pause echo enroll oblige weasel cliff hover";
//! ```
//!
//! ## Not flagged
//! ```js
//! const mnemonic = "remember to rotate the key";   // wrong word count / shape
//! const seedPhrase = "";                            // empty
//! const note = "abandon abandon ... about";         // target name not mnemonic
//! const seed = 12345;                               // not a string
//! ```
//!
//! ## Follow-up (out of scope here)
//! Mnemonics passed directly as call arguments or returned without a named
//! binding, and full BIP-39 dictionary validation, are documented follow-ups.

use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, BindingPattern, Expression, ObjectProperty,
    PropertyKey, VariableDeclarator,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "review-blockchain-mnemonic";

/// Whole-name (case-insensitive) match against the set of identifiers that
/// conventionally hold a wallet mnemonic/seed phrase. Both the compact and the
/// snake_case spellings are listed explicitly so the check is allocation-free.
fn is_mnemonic_name(name: &str) -> bool {
    [
        "mnemonic",
        "mnemonics",
        "seedphrase",
        "seed_phrase",
        "recoveryphrase",
        "recovery_phrase",
        "secretphrase",
        "secret_phrase",
        "backupphrase",
        "backup_phrase",
        "walletmnemonic",
        "wallet_mnemonic",
        "walletphrase",
        "wallet_phrase",
    ]
    .iter()
    .any(|w| name.eq_ignore_ascii_case(w))
}

/// True when `value` has the structural shape of a BIP-39 mnemonic: exactly
/// 12/15/18/21/24 whitespace-separated tokens, each 3–8 lowercase ASCII
/// letters.
fn looks_like_mnemonic(value: &str) -> bool {
    let mut count = 0u32;
    for token in value.split_whitespace() {
        let len = token.len();
        if !(3..=8).contains(&len) {
            return false;
        }
        if !token.bytes().all(|b| b.is_ascii_lowercase()) {
            return false;
        }
        count += 1;
        if count > 24 {
            return false;
        }
    }
    matches!(count, 12 | 15 | 18 | 21 | 24)
}

impl Scanner<'_> {
    pub(crate) fn check_review_blockchain_mnemonic_declarator(
        &mut self,
        it: &VariableDeclarator<'_>,
    ) {
        let name = match &it.id {
            BindingPattern::BindingIdentifier(ident) => ident.name.as_str(),
            _ => return,
        };
        if !is_mnemonic_name(name) {
            return;
        }
        let Some(Expression::StringLiteral(lit)) = &it.init else {
            return;
        };
        if !looks_like_mnemonic(lit.value.as_str()) {
            return;
        }
        self.report(RULE_NAME, "reviewMnemonic", lit.span);
    }

    pub(crate) fn check_review_blockchain_mnemonic_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let name = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if !is_mnemonic_name(name) {
            return;
        }
        let Expression::StringLiteral(lit) = &it.value else {
            return;
        };
        if !looks_like_mnemonic(lit.value.as_str()) {
            return;
        }
        self.report(RULE_NAME, "reviewMnemonic", lit.span);
    }

    pub(crate) fn check_review_blockchain_mnemonic_assignment(
        &mut self,
        it: &AssignmentExpression<'_>,
    ) {
        let name = match &it.left {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => ident.name.as_str(),
            AssignmentTarget::StaticMemberExpression(member) => member.property.name.as_str(),
            _ => return,
        };
        if !is_mnemonic_name(name) {
            return;
        }
        let Expression::StringLiteral(lit) = &it.right else {
            return;
        };
        if !looks_like_mnemonic(lit.value.as_str()) {
            return;
        }
        self.report(RULE_NAME, "reviewMnemonic", lit.span);
    }
}
