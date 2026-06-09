#![doc = "ESLint directive-comment rule logic for Rust-backed oxlint plugins."]
//!
//! Clean-room port of `@eslint-community/eslint-plugin-eslint-comments`. Each
//! rule consumes a per-file batch of comments and returns the diagnostics to
//! report, so the JavaScript wrapper makes a single NAPI call per file rather
//! than one call per AST node.

pub mod directive;
pub mod disabled_area;
pub mod loc;

mod rule_disable_enable_pair;
mod rule_no_aggregating_enable;
mod rule_no_duplicate_disable;
mod rule_no_unlimited_disable;
mod rule_no_use;
mod rule_require_description;

pub use loc::{Location, Position};
pub use rule_disable_enable_pair::disable_enable_pair;
pub use rule_no_aggregating_enable::no_aggregating_enable;
pub use rule_no_duplicate_disable::no_duplicate_disable;
pub use rule_no_unlimited_disable::no_unlimited_disable;
pub use rule_no_use::no_use;
pub use rule_require_description::require_description;

use oxlint_plugins_carton::CompactString;

use crate::directive::CommentKind;

/// An input comment for a per-file scan.
#[derive(Clone, Copy, Debug)]
pub struct Comment<'a> {
    pub kind: CommentKind,
    pub value: &'a str,
    pub loc: Location,
}

/// Values interpolated into a diagnostic's message template.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticData {
    pub kind: Option<CompactString>,
    pub rule_id: Option<CompactString>,
    pub count: Option<u32>,
}

/// A diagnostic produced by a rule, mapped to `context.report` by the wrapper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub message_id: CompactString,
    pub data: DiagnosticData,
    pub loc: Location,
}
