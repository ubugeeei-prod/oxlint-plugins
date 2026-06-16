//! Rule `hidden-files` (SonarJS key S5691).
//!
//! Clean-room port from public RSPEC S5691 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Static file servers can be configured to serve hidden files (dotfiles such
//! as `.env` or `.git`). These files frequently hold sensitive data such as
//! credentials or configuration, so exposing them over HTTP is a security
//! risk. The Express / serve-static middleware enables this when configured
//! with `dotfiles: 'allow'`; the safe values are `'ignore'` or `'deny'`.
//!
//! ## Zero-FP subset
//!
//! This port flags an `ObjectProperty` whose key (a static identifier or a
//! string literal) is exactly `dotfiles` and whose value is the string literal
//! `"allow"`. The `dotfiles: 'allow'` option is distinctive to
//! serve-static / express.static configuration, so flagging only this exact
//! shape is effectively zero-false-positive in practice. The property span is
//! reported.
//!
//! ## Flagged
//! ```js
//! serveStatic('public', { dotfiles: 'allow' }); // serve-static config
//! const x = { 'dotfiles': 'allow' };             // string-literal key
//! ```
//!
//! ## Not Flagged
//! ```js
//! const x = { dotfiles: 'ignore' }; // safe value
//! const x = { dotfiles: 'deny' };   // safe value
//! const x = { dotfiles: y };        // non-literal value
//! const x = { other: 'allow' };     // different key
//! ```

use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "hidden-files";

impl Scanner<'_> {
    pub(crate) fn check_hidden_files_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if key != "dotfiles" {
            return;
        }
        let is_allow = matches!(&it.value, Expression::StringLiteral(lit) if lit.value == "allow");
        if !is_allow {
            return;
        }
        self.report(RULE_NAME, "hiddenFiles", it.span);
    }
}
