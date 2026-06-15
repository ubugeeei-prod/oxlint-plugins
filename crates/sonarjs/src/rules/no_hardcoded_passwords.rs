//! Rule `no-hardcoded-passwords` (SonarJS key S2068).
//!
//! Clean-room port from public RSPEC S2068 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Detects bindings and assignments of a hardcoded string literal to an
//! identifier whose whole name matches a credential word. The credential-word
//! set (matched case-insensitively against the WHOLE name) is exactly:
//!
//!   `password`, `passwd`, `pwd`, `passphrase`
//!
//! ## Value guards (NOT flagged)
//!
//! The string value is excluded when it:
//!  - is shorter than 2 characters (including empty),
//!  - equals the target name itself case-insensitively (placeholder pattern,
//!    e.g. `password = "password"`), or
//!  - is one of a small fixed set of well-known placeholders:
//!    `"password"`, `"***"`, `"xxx"`, `"changeit"`, `"todo"`.
//!
//! ## Flagged
//! ```js
//! const password = "s3cr3t-value";            // VariableDeclarator
//! const config = { password: "hunter2abc" };  // ObjectProperty
//! obj.passwd = "realSecret123";               // AssignmentExpression (member)
//! pwd = "hardcoded!";                         // AssignmentExpression (ident)
//! ```
//!
//! ## Not Flagged
//! ```js
//! const password = "";                    // empty (too short)
//! const password = "password";           // value == name (placeholder)
//! const username = "admin";              // not a credential word
//! const password = getSecret();          // not a string literal
//! const passwordHint = "some hint";      // whole name is not a credential word
//! ```
//!
//! ## Follow-up (out of scope here)
//! URL-embedded credentials and call-argument forms are a documented follow-up
//! and are NOT handled in this initial port.

use oxc_ast::ast::{AssignmentTarget, Expression, ObjectProperty, PropertyKey, VariableDeclarator};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-hardcoded-passwords";

fn is_credential_word(name: &str) -> bool {
    ["password", "passwd", "pwd", "passphrase"]
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
    ["password", "***", "xxx", "changeit", "todo"]
        .iter()
        .any(|p| value.eq_ignore_ascii_case(p))
}

impl Scanner<'_> {
    pub(crate) fn check_no_hardcoded_passwords_declarator(&mut self, it: &VariableDeclarator<'_>) {
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
        self.report(RULE_NAME, "hardcodedPassword", span);
    }

    pub(crate) fn check_no_hardcoded_passwords_object_property(&mut self, it: &ObjectProperty<'_>) {
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
        self.report(RULE_NAME, "hardcodedPassword", span);
    }

    pub(crate) fn check_no_hardcoded_passwords_assignment(
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
        self.report(RULE_NAME, "hardcodedPassword", span);
    }
}
