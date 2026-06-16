//! Rule `content-length` (SonarJS key S5693).
//!
//! Clean-room port from public RSPEC S5693 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Accepting request bodies or file uploads that are too large lets an attacker
//! exhaust server resources, enabling a Denial-of-Service (DoS) attack. The
//! RSPEC recommends capping file uploads at 8MB (8,000,000 bytes) and other
//! request content at 2MB. A configured limit that is much larger than these
//! values (or no limit at all) is flagged as a sensitive sink.
//!
//! ## Zero-FP subset
//!
//! This port flags a numeric upload-size limit that is strictly greater than
//! 8MB (8,000,000), expressed in either of two distinctive forms:
//!
//! 1. An `ObjectProperty` whose key (a static identifier or string literal) is
//!    exactly `fileSize` or `maxFileSize` and whose value is a numeric literal
//!    `> 8_000_000`. These camelCase keys are distinctive to upload-size
//!    configuration (e.g. `multer({ limits: { fileSize: 10000000 } })`), so the
//!    check is effectively zero-false-positive. The property span is reported.
//!
//! 2. An `AssignmentExpression` whose left-hand side is a static member
//!    expression ending in `.maxFileSize` or `.fileSize` and whose right-hand
//!    side is a numeric literal `> 8_000_000` (e.g. the formidable form
//!    `form.maxFileSize = 10000000`). The assignment span is reported.
//!
//! ## Flagged
//! ```js
//! multer({ limits: { fileSize: 10000000 } });   // 10MB object property
//! const cfg = { maxFileSize: 9000000 };          // 9MB object property
//! form.maxFileSize = 10000000;                   // 10MB member assignment
//! ```
//!
//! ## Not flagged (deliberate under-reporting)
//! ```js
//! const cfg = { fileSize: 8000000 };   // exactly 8MB — within the limit
//! const cfg = { fileSize: 1000000 };   // 1MB — within the limit
//! const cfg = { fileSize: "4mb" };     // string with a unit — needs unit
//!                                      //   parsing; skipped to avoid FPs
//! const cfg = { limit: 10000000 };     // generic `limit` key — too common
//! const cfg = { fileSize: x };         // non-literal value — cannot prove
//! ```
//!
//! The `body-parser` `limit: "4mb"` string form, generic keys such as `limit`
//! / `maxSize`, and the missing-limit (library-default) case are intentionally
//! NOT flagged: each would require unit parsing or library-specific knowledge
//! that cannot be done without false positives. The rule therefore under-reports
//! relative to the full S5693 specification.

use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, Expression, ObjectProperty, PropertyKey,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "content-length";

/// Recommended maximum file-upload size in bytes (8MB). Limits strictly larger
/// than this are considered sensitive.
const MAX_FILE_SIZE_BYTES: f64 = 8_000_000.0;

fn is_upload_size_key(key: &str) -> bool {
    key == "fileSize" || key == "maxFileSize"
}

fn exceeds_limit(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::NumericLiteral(lit) if lit.value > MAX_FILE_SIZE_BYTES)
}

impl Scanner<'_> {
    pub(crate) fn check_content_length_object_property(&mut self, it: &ObjectProperty<'_>) {
        let key = match &it.key {
            PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
            PropertyKey::StringLiteral(lit) => lit.value.as_str(),
            _ => return,
        };
        if !is_upload_size_key(key) {
            return;
        }
        if !exceeds_limit(&it.value) {
            return;
        }
        self.report(RULE_NAME, "contentLength", it.span);
    }

    pub(crate) fn check_content_length_assignment(&mut self, it: &AssignmentExpression<'_>) {
        let AssignmentTarget::StaticMemberExpression(member) = &it.left else {
            return;
        };
        if !is_upload_size_key(member.property.name.as_str()) {
            return;
        }
        if !exceeds_limit(&it.right) {
            return;
        }
        self.report(RULE_NAME, "contentLength", it.span);
    }
}
