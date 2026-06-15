//! Rule `certificate-transparency` (SonarJS key S5742).
//!
//! Clean-room port from public RSPEC S5742 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Certificate Transparency (CT) lets clients detect misissued or fraudulent
//! TLS certificates by requiring them to be published to public, append-only
//! logs. The helmet middleware can emit an `Expect-CT` header that asks
//! browsers to enforce this. Setting helmet's `expectCt` option to `false`
//! disables that monitoring and removes a defence against certificate misuse.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `expectCt` and whose value is the boolean
//! literal `false`. The camelCase `expectCt` key is distinctive to helmet
//! configuration objects, so flagging only `expectCt: false` is effectively
//! zero-false-positive in practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! helmet({ expectCt: false });        // disables Expect-CT middleware
//! const x = { expectCt: false };      // direct config literal
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet({ expectCt: true });   // monitoring left enabled
//! const x = { expectCt: o };    // non-literal value (cannot prove unsafe)
//! const x = { other: false };   // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "certificate-transparency";

impl Scanner<'_> {
    pub(crate) fn check_certificate_transparency_object_property(
        &mut self,
        it: &ObjectProperty<'_>,
    ) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "expectCt" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "certificateTransparency", it.span);
    }
}
