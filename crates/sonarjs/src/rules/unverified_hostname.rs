//! Rule `unverified-hostname` (SonarJS key S5527).
//!
//! Clean-room port from public RSPEC S5527 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! During a TLS/SSL connection the client must verify that the certificate
//! presented by the server actually belongs to the hostname it is connecting
//! to. Node.js performs this check by default, but it can be replaced by
//! supplying a `checkServerIdentity` callback. If that callback never reports
//! an error, hostname verification is effectively disabled and an attacker who
//! holds any valid certificate can impersonate the server (man-in-the-middle).
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `checkServerIdentity` and whose value is a
//! function literal (a `FunctionExpression` or an `ArrowFunctionExpression`)
//! with a trivial "always pass" body. A body counts as trivial when it is:
//!
//! - an empty block `{}` (zero statements), or
//! - a block whose only statement is a bare `return;`, `return true;`, or
//!   `return undefined;`, or
//! - an arrow expression body that is just `true` (or `undefined`).
//!
//! The `checkServerIdentity` key combined with an empty / always-true override
//! is highly distinctive to TLS configuration, so flagging only this shape is
//! effectively zero-false-positive in practice. The property span is reported.
//!
//! If the body contains any other statement (an `if`, a `throw`, a comparison,
//! a non-trivial `return`, etc.) the override may be performing real
//! verification, so it is left alone. A non-function value (e.g. a reference to
//! a named function) is also left alone.
//!
//! ## Flagged
//! ```js
//! tls.connect({ checkServerIdentity: function() {} });   // empty body
//! const o = { checkServerIdentity: () => {} };           // empty arrow block
//! const o = { checkServerIdentity: () => true };         // always-true arrow
//! const o = { checkServerIdentity: function() { return; } };       // bare return
//! const o = { checkServerIdentity: function() { return true; } };  // return true
//! ```
//!
//! ## Not Flagged
//! ```js
//! const o = { checkServerIdentity: (h, c) => { if (h !== expected) throw new Error(); } }; // real logic
//! const o = { checkServerIdentity: someFn };  // not a function literal
//! const o = { other: function() {} };         // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey, Statement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "unverified-hostname";

/// Returns `true` if `expr` is a value that makes a `checkServerIdentity`
/// override trivially "always pass": the boolean literal `true` or the
/// `undefined` identifier.
fn is_always_pass_value(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::BooleanLiteral(b) => b.value,
        Expression::Identifier(id) => id.name == "undefined",
        _ => false,
    }
}

/// Returns `true` when a function block body is empty or contains only a single
/// trivial `return;` / `return true;` / `return undefined;` statement.
fn is_trivial_block(statements: &[Statement<'_>]) -> bool {
    match statements {
        [] => true,
        [Statement::ReturnStatement(ret)] => match &ret.argument {
            None => true,
            Some(arg) => is_always_pass_value(arg),
        },
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_unverified_hostname_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "checkServerIdentity" {
            return;
        }
        let is_trivial = match &it.value {
            Expression::FunctionExpression(func) => match &func.body {
                Some(body) => is_trivial_block(&body.statements),
                None => false,
            },
            Expression::ArrowFunctionExpression(arrow) => {
                if arrow.expression {
                    matches!(
                        arrow.body.statements.as_slice(),
                        [Statement::ExpressionStatement(es)] if is_always_pass_value(&es.expression)
                    )
                } else {
                    is_trivial_block(&arrow.body.statements)
                }
            }
            _ => return,
        };
        if !is_trivial {
            return;
        }
        self.report(RULE_NAME, "unverifiedHostname", it.span);
    }
}
