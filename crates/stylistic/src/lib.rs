#![allow(
    clippy::disallowed_types,
    reason = "Stylistic diagnostics and metadata are serialized over the JS ABI, and the imported source-wide scanner uses Vec/String for unbounded token streams."
)]

use std::collections::BTreeMap;

use oxlint_plugins_carton::SmallVec;
use serde::{Deserialize, Serialize};

#[allow(
    clippy::collapsible_if,
    clippy::unwrap_used,
    reason = "Imported Corsa stylistic scanner is kept behaviorally close to its source and has focused tests for these branches."
)]
mod native_stylistic;

pub use native_stylistic::{
    StylisticRuleConfig, StylisticRunConfig, run_stylistic_lint, stylistic_rule_metas,
};

/// UTF-8 byte range used by Oxlint-compatible diagnostics and fixes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRange {
    /// Inclusive byte offset where the range starts.
    pub start: u32,
    /// Exclusive byte offset where the range ends.
    pub end: u32,
}

impl TextRange {
    /// Creates a range from inclusive `start` and exclusive `end` offsets.
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Returns `true` when the range contains no bytes.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns `true` when `start` does not exceed `end`.
    pub const fn is_valid(self) -> bool {
        self.start <= self.end
    }
}

/// Text edit that can repair or rewrite a lint diagnostic range.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LintFix {
    /// Source byte range replaced by this fix.
    pub range: TextRange,
    /// Text inserted in place of [`Self::range`].
    pub replacement_text: String,
}

impl LintFix {
    /// Creates a fix that replaces `range` with `replacement_text`.
    pub fn replace_range(range: TextRange, replacement_text: impl Into<String>) -> Self {
        Self {
            range,
            replacement_text: replacement_text.into(),
        }
    }

    /// Creates a fix that removes `range`.
    pub fn remove_range(range: TextRange) -> Self {
        Self::replace_range(range, "")
    }
}

/// User-facing suggestion attached to a lint diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LintSuggestion {
    /// Message ID describing the suggested change.
    pub message_id: String,
    /// Rendered message text for the suggestion.
    pub message: String,
    /// Ordered fixes that implement the suggestion.
    pub fixes: Vec<LintFix>,
}

/// Serializable lint diagnostic returned to the host adapter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LintDiagnostic {
    /// Stable rule name that produced the diagnostic.
    pub rule_name: String,
    /// Stable rule-local message identifier.
    pub message_id: String,
    /// Rendered user-facing diagnostic message.
    pub message: String,
    /// Source byte range reported by the rule.
    pub range: TextRange,
    /// Optional suggestions for automated repair.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<LintSuggestion>,
}

/// Depth range for metadata the JavaScript host should attach to native nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetadataDepth {
    /// Minimum recursive node depth, where the listener node is depth 0.
    pub min_depth: u8,
    /// Maximum recursive node depth, inclusive.
    pub max_depth: u8,
}

impl NodeMetadataDepth {
    /// Includes metadata from the listener node through `max_depth`.
    pub const fn through(max_depth: u8) -> Self {
        Self {
            min_depth: 0,
            max_depth,
        }
    }
}

/// Host-side metadata requirements for a Rust-authored lint rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleBridgeRequirements {
    /// Maximum AST recursion depth the host should serialize.
    pub max_depth: u8,
    /// Depths where rendered type text should be attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_texts: Option<NodeMetadataDepth>,
    /// Depths where property names should be attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_names: Option<NodeMetadataDepth>,
    /// Depths where source text should be attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<NodeMetadataDepth>,
}

impl RuleBridgeRequirements {
    /// Default recursion depth used by the JavaScript host bridge.
    pub const DEFAULT_MAX_DEPTH: u8 = 4;

    /// Creates requirements for syntax-only rules.
    pub const fn syntax_only(max_depth: u8) -> Self {
        Self {
            max_depth,
            type_texts: None,
            property_names: None,
            text: None,
        }
    }

    /// Creates requirements that attach type text and property names together.
    pub const fn type_texts_and_properties(max_depth: u8, metadata: NodeMetadataDepth) -> Self {
        Self {
            max_depth,
            type_texts: Some(metadata),
            property_names: Some(metadata),
            text: None,
        }
    }

    /// Creates the default requirements for rules that only declare whether
    /// they need type text.
    pub const fn default_for_type_texts(requires_type_texts: bool) -> Self {
        if requires_type_texts {
            Self::type_texts_and_properties(
                Self::DEFAULT_MAX_DEPTH,
                NodeMetadataDepth::through(Self::DEFAULT_MAX_DEPTH),
            )
        } else {
            Self::syntax_only(Self::DEFAULT_MAX_DEPTH)
        }
    }
}

/// Serializable metadata that describes one Rust-authored lint rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleMeta {
    /// Stable rule name.
    pub name: String,
    /// Short documentation sentence for generated rule docs.
    pub docs_description: String,
    /// Message catalog keyed by message ID.
    pub messages: BTreeMap<String, String>,
    /// Whether the rule can emit suggestions.
    pub has_suggestions: bool,
    /// AST node kinds the rule listens to.
    pub listeners: Vec<String>,
    /// Whether the host should include rendered TypeScript type text.
    pub requires_type_texts: bool,
    /// Native bridge metadata requirements owned by the Rust rule registry.
    pub bridge: RuleBridgeRequirements,
}

pub const DEFAULT_FORBIDDEN_NAMES: &[&str] = &["event", "error", "data"];
static DEFAULT_FORBIDDEN_NAME_SET: phf::Set<&'static str> = phf::phf_set! {
    "event",
    "error",
    "data",
};

pub fn scan_source_for_rule<'a>(
    source_text: &str,
    custom_names: impl IntoIterator<Item = &'a str>,
) -> SmallVec<[&'a str; 8]> {
    let mut matches = SmallVec::new();

    for name in forbidden_names(custom_names) {
        if contains_identifier(source_text, name) {
            matches.push(name);
        }
    }

    matches
}

pub fn is_forbidden_identifier_name<'a>(
    name: &str,
    custom_names: impl IntoIterator<Item = &'a str>,
) -> bool {
    custom_names
        .into_iter()
        .filter(|custom_name| !custom_name.is_empty())
        .any(|forbidden| forbidden == name)
        || DEFAULT_FORBIDDEN_NAME_SET.contains(name)
}

pub fn contains_identifier(source_text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }

    let bytes = source_text.as_bytes();
    let mut offset = 0;

    while let Some(relative_start) = source_text[offset..].find(needle) {
        let start = offset + relative_start;
        let end = start + needle.len();

        if has_identifier_boundaries(bytes, start, end) {
            return true;
        }

        offset = end;
    }

    false
}

fn forbidden_names<'a>(
    custom_names: impl IntoIterator<Item = &'a str>,
) -> impl Iterator<Item = &'a str> {
    custom_names
        .into_iter()
        .filter(|name| !name.is_empty())
        .chain(DEFAULT_FORBIDDEN_NAMES.iter().copied())
}

fn has_identifier_boundaries(bytes: &[u8], start: usize, end: usize) -> bool {
    let before_is_identifier = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index))
        .is_some_and(|byte| is_ascii_identifier_continue(*byte));
    let after_is_identifier = bytes
        .get(end)
        .is_some_and(|byte| is_ascii_identifier_continue(*byte));

    !before_is_identifier && !after_is_identifier
}

fn is_ascii_identifier_continue(byte: u8) -> bool {
    byte == b'_' || byte == b'$' || byte.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::{contains_identifier, scan_source_for_rule};

    #[test]
    fn scans_default_names() {
        let source = "const event = data.error;";
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(scan_source_for_rule(source, []));
        }
    }

    #[test]
    fn supports_custom_names_without_losing_defaults() {
        let source = "function run(ctx) { return payload + event; }";
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(scan_source_for_rule(source, ["ctx", "payload"]));
        }
    }

    #[test]
    fn respects_identifier_boundaries() {
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(
                "identifier_boundaries",
                [
                    contains_identifier("const event = 1", "event"),
                    contains_identifier("const eventBus = 1", "event"),
                    contains_identifier("const my_event = 1", "event"),
                    contains_identifier("const $event = 1", "event"),
                    contains_identifier("call(event)", "event"),
                ]
            );
        }
    }
}
