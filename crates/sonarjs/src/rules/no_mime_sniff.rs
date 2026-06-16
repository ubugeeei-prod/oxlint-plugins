//! Rule `no-mime-sniff` (SonarJS key S5734).
//!
//! Clean-room port from public RSPEC S5734 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! The `X-Content-Type-Options: nosniff` response header tells browsers not to
//! guess (sniff) the MIME type of a resource, which protects against MIME
//! confusion attacks where, for example, an uploaded file disguised as an image
//! is interpreted and executed as a script. The `helmet` middleware sets this
//! header by default, so explicitly turning it off with
//! `helmet({ noSniff: false })` removes that protection.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `noSniff` and whose value is the boolean literal
//! `false`. The `noSniff` key is distinctive to helmet configuration objects,
//! so flagging only `noSniff: false` is effectively zero-false-positive in
//! practice. The property span is reported.
//!
//! ## Flagged
//! ```js
//! helmet({ noSniff: false }); // nosniff protection explicitly disabled
//! const o = { noSniff: false };
//! ```
//!
//! ## Not Flagged
//! ```js
//! helmet({ noSniff: true });  // explicitly enabled
//! const o = { noSniff: x };   // non-literal value
//! const o = { other: false }; // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-mime-sniff";

impl Scanner<'_> {
    pub(crate) fn check_no_mime_sniff_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "noSniff" {
            return;
        }
        let is_false = matches!(&it.value, Expression::BooleanLiteral(b) if !b.value);
        if !is_false {
            return;
        }
        self.report(RULE_NAME, "noMimeSniff", it.span);
    }
}
