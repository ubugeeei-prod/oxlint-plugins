//! Rule `no-vue-bypass-sanitization` (SonarJS key S6299).
//!
//! Clean-room port from the public RSPEC S6299 description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Vue.js automatically HTML-escapes interpolated content as a built-in defense
//! against Cross-Site Scripting (XSS). The spec ("Disabling Vue.js built-in
//! escaping is security-sensitive") flags the raw-HTML rendering APIs that turn
//! that escaping off, letting untrusted input reach the DOM verbatim. This port
//! flags the two distinctive, effectively zero-false-positive syntactic signals.
//!
//! ## Zero-FP subset — two shapes
//!
//! (a) A JSX attribute whose name is exactly `domPropsInnerHTML`. This Vue-JSX
//!     attribute name is highly distinctive and only ever means "render raw
//!     HTML, bypassing escaping". The attribute span is reported.
//!
//! (b) An `ObjectProperty` whose key (a static identifier or string literal) is
//!     exactly `domProps` and whose value is an object expression containing a
//!     property whose key is exactly `innerHTML`. This is the Vue render-function
//!     pattern `createElement('div', { domProps: { innerHTML: ... } })`. The
//!     `domProps` → `innerHTML` nesting is distinctive to Vue render functions.
//!     The inner `innerHTML` property's span is reported.
//!
//! ## Out of scope
//!
//! The spec's third form, the `v-html` template directive, lives in `.vue`
//! single-file-component template markup. This AST-based linter only parses
//! JavaScript/TypeScript/JSX, not Vue template HTML, so `v-html` is
//! intentionally not handled here.
//!
//! A bare `innerHTML` property that is not nested directly inside a `domProps`
//! object is deliberately NOT flagged: `innerHTML` is a ubiquitous DOM property
//! name and flagging it alone would be highly false-positive-prone.
//!
//! ## Flagged
//! ```jsx
//! <div domPropsInnerHTML={this.htmlContent}></div>                  // (a)
//! createElement('div', { domProps: { innerHTML: this.htmlContent } }); // (b)
//! ```
//!
//! ## Not flagged
//! ```jsx
//! <div innerHTML={x}></div>                  // not the domProps-JSX attribute
//! const o = { innerHTML: x };                // bare innerHTML, not under domProps
//! createElement('div', { attrs: { id: x } }); // domProps absent
//! ```

use oxc_ast::ast::{Expression, JSXAttribute, JSXAttributeName, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-vue-bypass-sanitization";

impl Scanner<'_> {
    pub(crate) fn check_no_vue_bypass_sanitization_jsx_attribute(&mut self, it: &JSXAttribute<'_>) {
        let JSXAttributeName::Identifier(name) = &it.name else {
            return;
        };
        if name.name.as_str() != "domPropsInnerHTML" {
            return;
        }
        self.report(RULE_NAME, "noVueBypassSanitization", it.span);
    }

    pub(crate) fn check_no_vue_bypass_sanitization_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "domProps" {
            return;
        }
        let Expression::ObjectExpression(object) = &it.value else {
            return;
        };
        for property in &object.properties {
            let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(inner) = property else {
                continue;
            };
            let inner_key = match &inner.key {
                PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                _ => continue,
            };
            if inner_key == "innerHTML" {
                self.report(RULE_NAME, "noVueBypassSanitization", inner.span);
            }
        }
    }
}
