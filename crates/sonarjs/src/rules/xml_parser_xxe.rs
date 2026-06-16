//! Rule `xml-parser-xxe` (SonarJS key S2755).
//!
//! Clean-room port from public RSPEC S2755 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An XML parser that has external entity expansion enabled is vulnerable to
//! XML External Entity (XXE) attacks, which can lead to sensitive data
//! exposure, denial of service, or Server-Side Request Forgery. With the
//! `libxmljs` parser, passing `{ noent: true }` to `parseXmlString` enables
//! entity substitution and exposes the application to XXE.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `noent` and whose value is the boolean literal
//! `true`. The `noent` option key is distinctive to libxmljs XML parsing, so
//! flagging only `noent: true` is effectively zero-false-positive in practice.
//! The property span is reported.
//!
//! ## Flagged
//! ```js
//! libxmljs.parseXmlString(xml, { noent: true }); // entity expansion enabled
//! const o = { noent: true };                      // direct config literal
//! ```
//!
//! ## Not Flagged
//! ```js
//! libxmljs.parseXmlString(xml, { noent: false }); // explicitly safe
//! const o = { noent: x };                          // non-literal value
//! const o = { other: true };                       // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "xml-parser-xxe";

impl Scanner<'_> {
    pub(crate) fn check_xml_parser_xxe_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "noent" {
            return;
        }
        let is_true = matches!(&it.value, Expression::BooleanLiteral(b) if b.value);
        if !is_true {
            return;
        }
        self.report(RULE_NAME, "xmlParserXxe", it.span);
    }
}
