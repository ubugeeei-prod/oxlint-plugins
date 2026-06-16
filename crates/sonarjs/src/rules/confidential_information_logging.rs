//! Rule `confidential-information-logging` (SonarJS key S5757).
//!
//! Clean-room port from the public RSPEC S5757 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Logs frequently flow into external SIEM or analytics platforms, so they must
//! never carry secrets such as passwords or credit-card numbers. The `signale`
//! logger supports a `secrets` option: a list of patterns it masks before
//! writing each entry. Constructing the logger with an empty `secrets` list
//! disables that masking entirely, allowing confidential data to be logged in
//! clear text.
//!
//! ## Zero-FP subset
//!
//! This port flags a `NewExpression` whose callee (after stripping wrapping
//! parentheses/non-null assertions via `get_inner_expression`) is the
//! distinctive `Signale` class — either a bare identifier `Signale` or a static
//! member expression ending in `.Signale` — whose first argument is an object
//! literal containing a property whose key is exactly `secrets` and whose value
//! is an empty array literal (`[]`, zero elements). The `Signale` class paired
//! with an empty `secrets` list is specific enough to the `signale` logging API
//! that flagging only this exact shape stays effectively false-positive free.
//! The new-expression span is reported.
//!
//! ## Flagged
//! ```js
//! new Signale({ secrets: [] });
//! ```
//!
//! ## Not flagged
//! ```js
//! new Signale({ secrets: ["password"] }); // non-empty patterns
//! new Signale({});                         // no secrets key
//! new Signale({ secrets: x });             // non-array value
//! new Other({ secrets: [] });              // different callee
//! ```

use oxc_ast::ast::{Argument, Expression, NewExpression, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "confidential-information-logging";

impl<'a> Scanner<'a> {
    /// Reports a `new Signale({ secrets: [] })` whose empty `secrets` list
    /// disables secret masking, allowing confidential data to be logged.
    pub(crate) fn check_confidential_information_logging(&mut self, expr: &NewExpression<'a>) {
        let is_signale = match expr.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident.name.as_str() == "Signale",
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "Signale"
            }
            _ => false,
        };
        if !is_signale {
            return;
        }
        let Some(Argument::ObjectExpression(options)) = expr.arguments.first() else {
            return;
        };
        let has_empty_secrets = options.properties.iter().any(|property| {
            let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(prop) = property else {
                return false;
            };
            let key = match &prop.key {
                PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                _ => return false,
            };
            if key != "secrets" {
                return false;
            }
            matches!(&prop.value, Expression::ArrayExpression(array) if array.elements.is_empty())
        });
        if has_empty_secrets {
            self.report(RULE_NAME, "confidentialLogging", expr.span);
        }
    }
}
