//! Rule `encryption` (SonarJS key S4787).
//!
//! Clean-room port. Behaviour is reproduced from the public RSPEC S4787
//! description ("Encrypting data is security-sensitive") only; no upstream
//! source, tests, fixtures, helper code, or message strings were consulted or
//! copied.
//!
//! S4787 is a *security hotspot* (and is now **deprecated** upstream, but it
//! remains part of the plugin, so it is ported here): encrypting or decrypting
//! data is not a bug in itself, but each such call should be reviewed to make
//! sure the algorithm, key strength, initialization vector (IV), and padding
//! are chosen safely. The rule does not analyse the arguments; it surfaces the
//! distinctive encryption/decryption call shapes for manual review. The public
//! RSPEC lists these sensitive examples (verbatim):
//!
//! ```js
//! crypto.subtle.encrypt(algo, key, plainData);
//! crypto.subtle.decrypt(algo, key, encData);
//! crypto.createCipher(algo, key);
//! crypto.createCipheriv(algo, key, iv);
//! crypto.createDecipher(algo, key);
//! crypto.createDecipheriv(algo, key, iv);
//! crypto.publicEncrypt(key, buf);
//! crypto.privateDecrypt({ key, passphrase }, encrypted);
//! crypto.privateEncrypt({ key, passphrase }, buf);
//! crypto.publicDecrypt(key, encrypted);
//! ```
//!
//! **Narrowing (zero-false-positive subset)**:
//! A `CallExpression` is flagged when its callee (after unwrapping parentheses
//! via `get_inner_expression`) is a `StaticMemberExpression` whose property
//! name matches one of two groups:
//!
//! - **Group A — distinctive Node `crypto` names, matched on any receiver:**
//!   `createCipher`, `createCipheriv`, `createDecipher`, `createDecipheriv`,
//!   `publicEncrypt`, `privateDecrypt`, `privateEncrypt`, `publicDecrypt`.
//!   These names are specific enough to the crypto module that they are flagged
//!   regardless of the object they are called on.
//!
//! - **Group B — generic `encrypt` / `decrypt`, receiver-gated on `.subtle`:**
//!   the property name is `encrypt` or `decrypt` AND the member's *object*
//!   (after `get_inner_expression`) is itself a `StaticMemberExpression` whose
//!   property name is `subtle`. This matches `crypto.subtle.encrypt(...)` and
//!   `window.crypto.subtle.encrypt(...)` (the Web Crypto `SubtleCrypto` API)
//!   while leaving the far-too-generic bare `encrypt`/`decrypt` on any other
//!   receiver alone.
//!
//! **Flagged**:
//! - `crypto.createCipheriv(algo, key, iv)` / `crypto.createDecipheriv(...)`
//! - `crypto.createCipher(algo, key)` / `crypto.createDecipher(algo, key)`
//! - `crypto.publicEncrypt(key, buf)` / `crypto.publicDecrypt(key, encrypted)`
//! - `crypto.privateEncrypt({ key }, buf)` / `crypto.privateDecrypt({ key }, e)`
//! - `crypto.subtle.encrypt(algo, key, data)` / `crypto.subtle.decrypt(...)`
//! - `window.crypto.subtle.encrypt(algo, key, data)`
//!
//! **Not flagged**:
//! - `obj.encrypt(data)` / `service.decrypt(data)` — `encrypt`/`decrypt` on a
//!   non-`.subtle` receiver (the names are far too generic).
//! - `crypto.randomBytes(16)` / `crypto.createHash("sha256")` — unrelated
//!   crypto methods.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "encryption";

impl Scanner<'_> {
    /// Reports data-encryption / decryption calls (security hotspot S4787): the
    /// distinctive Node `crypto` functions on any receiver, plus the generic
    /// `encrypt`/`decrypt` gated on a `.subtle` receiver.
    pub(crate) fn check_encryption(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        let property = member.property.name.as_str();
        let is_encryption = match property {
            // Group A — distinctive Node `crypto` names, any receiver.
            "createCipher" | "createCipheriv" | "createDecipher" | "createDecipheriv"
            | "publicEncrypt" | "privateDecrypt" | "privateEncrypt" | "publicDecrypt" => true,
            // Group B — generic `encrypt`/`decrypt`, only on a `.subtle` receiver.
            "encrypt" | "decrypt" => {
                matches!(
                    member.object.get_inner_expression(),
                    Expression::StaticMemberExpression(object)
                        if object.property.name == "subtle"
                )
            }
            _ => false,
        };
        if is_encryption {
            self.report(RULE_NAME, "encryption", it.span);
        }
    }
}
