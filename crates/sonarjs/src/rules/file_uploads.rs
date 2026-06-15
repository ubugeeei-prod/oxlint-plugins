//! Rule `file-uploads` (SonarJS key S2598).
//!
//! Clean-room port. When a Node.js application accepts file uploads it should
//! restrict where those files are written. With the popular `multer` library,
//! configuring `multer.diskStorage({ ... })` without a `destination` property
//! makes uploaded files land in the operating-system temporary directory — a
//! world-shared location — which is a security hotspot.
//!
//! This implements ONLY the unambiguous, zero-false-positive subset built on
//! multer's distinctively named `diskStorage` factory: a call whose callee is a
//! static member expression named `diskStorage`, whose first argument is an
//! object literal that has NO `destination` property. The `diskStorage` name is
//! distinctive enough that matching on it alone (regardless of the receiver)
//! stays effectively zero-false-positive while catching `multer.diskStorage`
//! and aliased imports such as `m.diskStorage` without type analysis.
//!
//! Formidable's `uploadDir` hotspot from the same RSPEC requires tracking a
//! `new Formidable()` instance across later property assignments (dataflow) and
//! is deliberately out of scope.
//!
//! ## Flagged
//! ```js
//! multer.diskStorage({ filename: fn });   // no destination -> OS temp dir
//! m.diskStorage({ filename: fn });         // aliased receiver, still no dest
//! ```
//!
//! ## Not flagged
//! ```js
//! multer.diskStorage({ destination: "/up", filename: fn }); // explicit dest
//! multer.diskStorage({ ["destination"]: d });               // string-key dest
//! multer.diskStorage();                                     // no object arg
//! multer.diskStorage(opts);                                 // non-literal arg
//! foo.bar({ filename: fn });                                // not diskStorage
//! bar();                                                    // unrelated call
//! ```
//!
//! Note: `diskStorage()` with no arguments (or a non-object first argument) is
//! deliberately SKIPPED — without an object literal there is nothing to inspect
//! and reporting could over-report.
//!
//! Behaviour is reproduced from the public RSPEC S2598 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "file-uploads";

impl Scanner<'_> {
    pub(crate) fn check_file_uploads(&mut self, expr: &oxc_ast::ast::CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        if member.property.name != "diskStorage" {
            return;
        }
        // Require an object literal as the first argument; without one there is
        // nothing to inspect, so skip to stay zero-false-positive.
        let Some(first) = expr.arguments.first().and_then(|arg| arg.as_expression()) else {
            return;
        };
        let Expression::ObjectExpression(object) = first.get_inner_expression() else {
            return;
        };
        if has_destination_property(object) {
            return;
        }
        self.report(RULE_NAME, "fileUploads", expr.span);
    }
}

/// Returns `true` when `object` has an own property keyed by the identifier or
/// string literal `destination`.
fn has_destination_property(object: &oxc_ast::ast::ObjectExpression<'_>) -> bool {
    object.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(prop) = property else {
            return false;
        };
        match &prop.key {
            PropertyKey::StaticIdentifier(ident) => ident.name == "destination",
            PropertyKey::StringLiteral(lit) => lit.value == "destination",
            _ => false,
        }
    })
}
