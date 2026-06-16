//! Rule `no-uniq-key` (SonarJS key S6486).
//!
//! Clean-room port of a verified zero-false-positive subset. The full upstream
//! rule keeps React list `key` attributes stable across renders, which in
//! general needs data-flow analysis. This port flags only the distinctive,
//! always-wrong syntactic case: a JSX `key` attribute whose value is a call to
//! `Math.random()` or `Date.now()`. Both produce a different value on every
//! render, so the key never matches up between renders and React is forced to
//! recreate the DOM — a bug regardless of whether the element sits in a `.map`.
//!
//! Behaviour derived from the public RSPEC S6486 description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//!
//! ```jsx
//! <li key={Math.random()}>x</li>
//! <li key={Date.now()}>x</li>
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! <li key={item.id}>x</li>      // stable identifier
//! <li key={index}>x</li>        // index-as-key needs data flow (out of scope)
//! <li key={`prefix-${i}`}>x</li> // template, not a random/time call
//! <li key="static">x</li>       // string literal
//! <li id={Math.random()}>x</li> // not the `key` attribute
//! ```
//!
//! Index-as-key and cross-render key mismatches require data-flow analysis and
//! are intentionally not reported here (documented under-report).

use oxc_ast::ast::{Expression, JSXAttribute, JSXAttributeName, JSXAttributeValue, JSXExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-uniq-key";

impl Scanner<'_> {
    pub(crate) fn check_no_uniq_key_jsx_attribute(&mut self, attr: &JSXAttribute<'_>) {
        // Only the `key` attribute is relevant.
        let JSXAttributeName::Identifier(name) = &attr.name else {
            return;
        };
        if name.name.as_str() != "key" {
            return;
        }

        // The value must be an expression container `key={...}`.
        let Some(JSXAttributeValue::ExpressionContainer(container)) = &attr.value else {
            return;
        };

        // The contained expression must be a call expression.
        let JSXExpression::CallExpression(call) = &container.expression else {
            return;
        };

        // The callee must be `Math.random` or `Date.now`.
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        let property = member.property.name.as_str();
        let is_random_or_time = matches!(
            (object.name.as_str(), property),
            ("Math", "random") | ("Date", "now")
        );
        if !is_random_or_time {
            return;
        }

        self.report(RULE_NAME, "noUniqKey", attr.span);
    }
}
