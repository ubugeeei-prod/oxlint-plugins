//! Rule `no-hardcoded-secrets` (SonarJS key S6418).
//!
//! Clean-room port from public RSPEC S6418 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied. This
//! rule is the sibling of `no-hardcoded-passwords` and mirrors its structure
//! exactly, differing only in the credential-word set, rule name, and message.
//!
//! Detects bindings and assignments of a hardcoded string literal to an
//! identifier whose whole name matches a secret/credential word. The
//! credential-word set (matched case-insensitively against the WHOLE name) is
//! exactly:
//!
//!   `secret`, `secrets`, `apikey`, `api_key`, `apiKey`, `token`,
//!   `auth_token`, `authtoken`, `accesstoken`, `access_token`, `credential`,
//!   `credentials`, `privatekey`, `private_key`
//!
//! Matching is whole-name only (no substring matching), so names such as
//! `tokenizer` or `tokenCount` are NOT credential words.
//!
//! ## Value guards (NOT flagged)
//!
//! The string value is excluded when it:
//!  - is shorter than 2 characters (including empty),
//!  - equals the target name itself case-insensitively (placeholder pattern,
//!    e.g. `secret = "secret"`), or
//!  - is one of a small fixed set of well-known placeholders:
//!    `"token"`, `"secret"`, `"changeit"`, `"xxx"`, `"undefined"`, `"null"`,
//!    `"none"`, `"your-api-key"`, `"password"`, `"***"`, `"todo"`.
//!
//! ## Flagged
//! ```js
//! const apiKey = "AKIA1234567890ABCD";        // VariableDeclarator
//! const config = { secret: "s3cr3tVal" };     // ObjectProperty
//! obj.token = "ghp_realLongTokenValue123";    // AssignmentExpression (member)
//! access_token = "hardcoded-value-123";       // AssignmentExpression (ident)
//! ```
//!
//! ## Not Flagged
//! ```js
//! const apiKey = "";                     // empty (too short)
//! const apiKey = "token";                // well-known placeholder
//! const username = "admin";              // not a credential word
//! const apiKey = process.env.KEY;        // not a string literal
//! const tokenizer = "x";                 // whole name is not a credential word
//! ```
//!
//! ## Follow-up (out of scope here)
//! URL-embedded credentials and call-argument forms are a documented follow-up
//! and are NOT handled in this initial port.

use oxc_ast::ast::{AssignmentTarget, Expression, ObjectProperty, PropertyKey, VariableDeclarator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-hardcoded-secrets";

fn is_credential_word(name: &str) -> bool {
    [
        "secret",
        "secrets",
        "apikey",
        "api_key",
        "apiKey",
        "token",
        "auth_token",
        "authtoken",
        "accesstoken",
        "access_token",
        "credential",
        "credentials",
        "privatekey",
        "private_key",
    ]
    .iter()
    .any(|w| name.eq_ignore_ascii_case(w))
}

fn is_trivial_value(value: &str, name: &str) -> bool {
    if value.len() < 2 {
        return true;
    }
    if value.eq_ignore_ascii_case(name) {
        return true;
    }
    [
        "token",
        "secret",
        "changeit",
        "xxx",
        "undefined",
        "null",
        "none",
        "your-api-key",
        "password",
        "***",
        "todo",
    ]
    .iter()
    .any(|p| value.eq_ignore_ascii_case(p))
}

impl Scanner<'_> {
    pub(crate) fn check_no_hardcoded_secrets_declarator(&mut self, it: &VariableDeclarator<'_>) {
        use oxc_ast::ast::BindingPattern;

        let name = match &it.id {
            BindingPattern::BindingIdentifier(ident) => ident.name.as_str(),
            _ => return,
        };
        if !is_credential_word(name) {
            return;
        }
        let init = match &it.init {
            Some(init) => init,
            None => return,
        };
        let (value, span) = match init {
            Expression::StringLiteral(lit) => (lit.value.as_str(), lit.span),
            _ => return,
        };
        if is_trivial_value(value, name) {
            return;
        }
        self.report(RULE_NAME, "hardcodedSecret", span);
    }

    pub(crate) fn check_no_hardcoded_secrets_object_property(&mut self, it: &ObjectProperty<'_>) {
        let name = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if !is_credential_word(name) {
            return;
        }
        let (value, span) = match &it.value {
            Expression::StringLiteral(lit) => (lit.value.as_str(), lit.span),
            _ => return,
        };
        if is_trivial_value(value, name) {
            return;
        }
        self.report(RULE_NAME, "hardcodedSecret", span);
    }

    pub(crate) fn check_no_hardcoded_secrets_assignment(
        &mut self,
        it: &oxc_ast::ast::AssignmentExpression<'_>,
    ) {
        let name = match &it.left {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => ident.name.as_str(),
            AssignmentTarget::StaticMemberExpression(member) => member.property.name.as_str(),
            _ => return,
        };
        if !is_credential_word(name) {
            return;
        }
        let (value, span) = match &it.right {
            Expression::StringLiteral(lit) => (lit.value.as_str(), lit.span),
            _ => return,
        };
        if is_trivial_value(value, name) {
            return;
        }
        self.report(RULE_NAME, "hardcodedSecret", span);
    }
}
