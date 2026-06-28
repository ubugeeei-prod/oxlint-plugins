//! Adapter for the `eslint-comments` plugin (port of
//! @eslint-community/eslint-plugin-eslint-comments).
//!
//! The core works on the file's comments (and, for `no-unused-disable`, the
//! other rules' problems) rather than on source text, so this adapter parses
//! with oxc to recover the comment list and first-token position the npm
//! wrapper normally gets from `sourceCode`.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_eslint_comments::directive::CommentKind;
use oxlint_plugins_eslint_comments::{
    Comment, Diagnostic as CoreDiagnostic, Location, Position, Problem, disable_enable_pair,
    no_aggregating_enable, no_duplicate_disable, no_restricted_disable, no_unlimited_disable,
    no_unused_disable, no_unused_enable, no_use, require_description,
};

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "eslint-comments";

// The core crate exposes no rule-name accessor; mirror the names registered in
// `npm/eslint-comments/index.js`.
const RULE_NAMES: [&str; 9] = [
    "disable-enable-pair",
    "no-aggregating-enable",
    "no-duplicate-disable",
    "no-restricted-disable",
    "no-unlimited-disable",
    "no-unused-disable",
    "no-unused-enable",
    "no-use",
    "require-description",
];

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: RULE_NAMES.iter().map(|name| (*name).to_owned()).collect(),
    }
}

pub fn scan(
    source_text: &str,
    filename: &str,
    filter: &EnabledFilter,
    out: &mut Vec<PlaygroundDiagnostic>,
) {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let parsed = Parser::new(&allocator, source_text, source_type).parse();

    let line_index = LineIndex::new(source_text);
    let comments: Vec<Comment> = parsed
        .program
        .comments
        .iter()
        .map(|comment| {
            let content = comment.content_span();
            let value = source_text
                .get(content.start as usize..content.end as usize)
                .unwrap_or("");
            Comment {
                kind: if comment.is_line() {
                    CommentKind::Line
                } else {
                    CommentKind::Block
                },
                value,
                loc: Location {
                    start: line_index.position(source_text, comment.span.start),
                    end: line_index.position(source_text, comment.span.end),
                },
            }
        })
        .collect();
    if comments.is_empty() {
        return;
    }

    let enabled = |rule: &str| filter.rule_enabled(PLUGIN, rule);
    let mut results: Vec<PlaygroundDiagnostic> = Vec::new();

    if enabled("no-unlimited-disable") {
        collect(
            &mut results,
            "no-unlimited-disable",
            no_unlimited_disable(&comments),
        );
    }
    // No options UI: every option-bearing rule runs with upstream's defaults
    // (empty allow/ignore/patterns, `allowWholeFile: false`).
    if enabled("no-use") {
        collect(&mut results, "no-use", no_use(&comments, &[]));
    }
    if enabled("require-description") {
        collect(
            &mut results,
            "require-description",
            require_description(&comments, &[]),
        );
    }
    if enabled("no-aggregating-enable") {
        collect(
            &mut results,
            "no-aggregating-enable",
            no_aggregating_enable(&comments),
        );
    }
    if enabled("no-duplicate-disable") {
        collect(
            &mut results,
            "no-duplicate-disable",
            no_duplicate_disable(&comments),
        );
    }
    if enabled("no-unused-enable") {
        collect(
            &mut results,
            "no-unused-enable",
            no_unused_enable(&comments),
        );
    }
    if enabled("no-restricted-disable") {
        collect(
            &mut results,
            "no-restricted-disable",
            no_restricted_disable(&comments, &[]),
        );
    }
    if enabled("disable-enable-pair") {
        let first_token = first_token_position(source_text, &parsed, &line_index);
        collect(
            &mut results,
            "disable-enable-pair",
            disable_enable_pair(&comments, false, first_token),
        );
    }
    if enabled("no-unused-disable") {
        // The playground's analog of `sourceCode.getDisableDirectives().problems`
        // is every diagnostic the other enabled plugins already reported.
        let rule_ids: Vec<String> = out
            .iter()
            .map(|diagnostic| {
                let mut id =
                    String::with_capacity(diagnostic.plugin.len() + 1 + diagnostic.rule.len());
                id.push_str(diagnostic.plugin);
                id.push('/');
                id.push_str(&diagnostic.rule);
                id
            })
            .collect();
        let problems: Vec<Problem> = out
            .iter()
            .zip(&rule_ids)
            .map(|(diagnostic, id)| Problem {
                rule_id: Some(id.as_str()),
                position: Position {
                    line: diagnostic.start_line,
                    column: diagnostic.start_column as i32,
                },
            })
            .collect();
        collect(
            &mut results,
            "no-unused-disable",
            no_unused_disable(&comments, &problems),
        );
    }

    out.append(&mut results);
}

/// Converts core diagnostics for `rule` into playground diagnostics.
fn collect(
    out: &mut Vec<PlaygroundDiagnostic>,
    rule: &str,
    diagnostics: impl IntoIterator<Item = CoreDiagnostic>,
) {
    for diagnostic in diagnostics {
        let mut data: BTreeMap<&'static str, String> = BTreeMap::new();
        if let Some(kind) = diagnostic.data.kind {
            data.insert("kind", kind.as_str().to_owned());
        }
        if let Some(rule_id) = diagnostic.data.rule_id {
            data.insert("ruleId", rule_id.as_str().to_owned());
        }
        if let Some(count) = diagnostic.data.count {
            data.insert("count", count.to_string());
        }
        out.push(PlaygroundDiagnostic {
            plugin: PLUGIN,
            rule: rule.to_owned(),
            message_id: diagnostic.message_id.as_str().to_owned(),
            data,
            start_line: diagnostic.loc.start.line,
            start_column: column_to_u32(diagnostic.loc.start.column),
            end_line: diagnostic.loc.end.line,
            end_column: column_to_u32(diagnostic.loc.end.column),
        });
    }
}

/// Clamps the core's `i32` column (which uses `-1` as a "whole line" sentinel)
/// to a non-negative value the editor can place.
fn column_to_u32(column: i32) -> u32 {
    column.max(0) as u32
}

/// The position of the first non-comment, non-whitespace token, mirroring
/// `sourceCode.ast.tokens[0]`. `None` for a comment-only or empty file.
fn first_token_position(
    source_text: &str,
    parsed: &oxc_parser::ParserReturn,
    line_index: &LineIndex,
) -> Option<Position> {
    let bytes = source_text.as_bytes();
    let len = bytes.len();
    let mut offset = 0usize;
    loop {
        while offset < len && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }
        if offset >= len {
            return None;
        }
        if let Some(comment) = parsed.program.comments.iter().find(|comment| {
            (comment.span.start as usize) <= offset && offset < comment.span.end as usize
        }) {
            offset = comment.span.end as usize;
            continue;
        }
        return Some(line_index.position(source_text, offset as u32));
    }
}

/// Byte-offset → 1-based line / 0-based UTF-16 column index, matching ESLint's
/// comment locations.
struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn new(source_text: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn position(&self, source_text: &str, offset: u32) -> Position {
        let offset = (offset as usize).min(source_text.len());
        let line_index = self
            .line_starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts.get(line_index).copied().unwrap_or(0);
        let column = source_text
            .get(line_start..offset)
            .unwrap_or("")
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        Position {
            line: (line_index + 1) as u32,
            column: column as i32,
        }
    }
}
