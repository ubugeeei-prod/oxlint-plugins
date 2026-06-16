//! Rule `disabled-auto-escaping` (SonarJS key S5247).
//!
//! Clean-room port from public RSPEC S5247 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Template engines auto-escape dynamic variables (converting characters such
//! as `<` and `>` into safe HTML entities) as a key defense against Cross-Site
//! Scripting (XSS). Turning that escaping off lets untrusted input reach the
//! rendered markup verbatim. This port flags the two distinctive, effectively
//! zero-false-positive signals from the spec.
//!
//! ## Zero-FP subset
//!
//! (a) An `ObjectProperty` whose key (a static identifier or string literal)
//!     is exactly `noEscape` with the boolean-literal value `true` — the
//!     Handlebars `compile` option that disables HTML escaping. The camelCase
//!     `noEscape` key is distinctive to template configuration.
//!
//! (b) An `AssignmentExpression` whose left-hand side is the static member
//!     expression `Mustache.escape` (object identifier `Mustache`, property
//!     `escape`) — overriding mustache.js's built-in escaper, which is the
//!     documented way to defeat its auto-escaping.
//!
//! ## Deliberately NOT flagged (judged too false-positive-prone)
//!
//! The spec also lists `html: true` (markdown-it) and `sanitize: false`
//! (marked / kramed). Both `html` and `sanitize` are extremely common generic
//! option names across unrelated libraries, so flagging them on key+value
//! alone (without knowing the receiving API) would produce false positives.
//! They are intentionally omitted.
//!
//! ## Flagged
//! ```js
//! Handlebars.compile(source, { noEscape: true });   // (a)
//! Mustache.escape = function (text) { return text; }; // (b)
//! ```
//!
//! ## Not Flagged
//! ```js
//! Handlebars.compile(source, { noEscape: false }); // escaping kept on
//! Handlebars.compile(source, { noEscape: flag });  // non-literal value
//! md({ html: true });                              // generic key, skipped
//! ```

use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, Expression, ObjectProperty, PropertyKey,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "disabled-auto-escaping";

impl Scanner<'_> {
    pub(crate) fn check_disabled_auto_escaping_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "noEscape" {
            return;
        }
        let is_true = matches!(&it.value, Expression::BooleanLiteral(b) if b.value);
        if !is_true {
            return;
        }
        self.report(RULE_NAME, "disabledAutoEscaping", it.span);
    }

    pub(crate) fn check_disabled_auto_escaping_assignment(
        &mut self,
        assign: &AssignmentExpression<'_>,
    ) {
        let AssignmentTarget::StaticMemberExpression(member) = &assign.left else {
            return;
        };
        if member.property.name.as_str() != "escape" {
            return;
        }
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        if object.name.as_str() != "Mustache" {
            return;
        }
        self.report(RULE_NAME, "disabledAutoEscaping", assign.span);
    }
}
