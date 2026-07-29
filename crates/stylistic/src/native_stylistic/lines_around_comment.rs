//! Native implementation of `@stylistic/lines-around-comment`.
//!
//! Comment discovery stays on the shared token scan. Oxc's AST is used only
//! for the rule's `allow*Start` / `allow*End` options because those options are
//! explicitly defined in terms of the comment's immediate parent node.

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use regex::Regex;
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::{
    context::{Scan, first_option, option_object_bool},
    lexer::TokenKind,
};

const RULE_NAME: &str = "lines-around-comment";
const BEFORE_MESSAGE: &str = "Expected line before comment.";
const AFTER_MESSAGE: &str = "Expected line after comment.";
const INSERT_LINE_MESSAGE: &str = "Insert an empty line.";

const BLOCK: u16 = 1 << 0;
const CLASS: u16 = 1 << 1;
const OBJECT: u16 = 1 << 2;
const ARRAY: u16 = 1 << 3;
const INTERFACE: u16 = 1 << 4;
const TYPE: u16 = 1 << 5;
const ENUM: u16 = 1 << 6;
const MODULE: u16 = 1 << 7;

#[derive(Clone, Copy)]
struct NodeRecord {
    span: Span,
    targets: u16,
    boundary_start: u32,
    static_open: Option<u32>,
}

struct ParentCollector {
    nodes: Vec<NodeRecord>,
}

impl<'ast> Visit<'ast> for ParentCollector {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        let span = kind.span();
        let (targets, boundary_start, static_open) = match kind {
            AstKind::ClassBody(_) => (BLOCK | CLASS, span.start, None),
            AstKind::BlockStatement(_) | AstKind::FunctionBody(_) => (BLOCK, span.start, None),
            AstKind::StaticBlock(_) => (BLOCK, span.start, Some(span.start)),
            AstKind::SwitchCase(_) => (BLOCK, span.start, None),
            AstKind::SwitchStatement(statement) => (BLOCK, statement.discriminant.span().end, None),
            AstKind::ObjectExpression(_) | AstKind::ObjectPattern(_) => (OBJECT, span.start, None),
            AstKind::ArrayExpression(_) | AstKind::ArrayPattern(_) => (ARRAY, span.start, None),
            AstKind::TSInterfaceBody(_) => (INTERFACE, span.start, None),
            AstKind::TSTypeLiteral(_) => (TYPE, span.start, None),
            AstKind::TSEnumBody(_) | AstKind::TSEnumDeclaration(_) => (ENUM, span.start, None),
            AstKind::TSModuleBlock(_) => (MODULE, span.start, None),
            _ => (0, span.start, None),
        };
        self.nodes.push(NodeRecord {
            span,
            targets,
            boundary_start,
            static_open,
        });
    }
}

#[derive(Clone, Copy)]
struct PhysicalLine {
    start: usize,
    content_end: usize,
}

struct Lines {
    lines: Vec<PhysicalLine>,
    blank: Vec<bool>,
    comment: Vec<bool>,
}

impl Lines {
    fn new(source: &str) -> Self {
        let mut lines = Vec::new();
        let mut start = 0;
        let mut cursor = 0;
        let bytes = source.as_bytes();

        while cursor < bytes.len() {
            let newline_len = newline_len_at(bytes, cursor);
            if newline_len == 0 {
                cursor += 1;
                continue;
            }
            lines.push(PhysicalLine {
                start,
                content_end: cursor,
            });
            cursor += newline_len;
            start = cursor;
        }
        lines.push(PhysicalLine {
            start,
            content_end: source.len(),
        });

        let blank = lines
            .iter()
            .map(|line| source[line.start..line.content_end].trim().is_empty())
            .collect::<Vec<_>>();
        let comment = std::iter::repeat_n(false, lines.len()).collect::<Vec<_>>();
        Self {
            lines,
            blank,
            comment,
        }
    }

    fn line_index(&self, offset: usize) -> usize {
        self.lines
            .partition_point(|line| line.start <= offset)
            .saturating_sub(1)
    }

    fn mark_comment(&mut self, start: usize, end: usize) {
        let start_line = self.line_index(start);
        let end_line = self.line_index(end.saturating_sub(1).max(start));
        self.comment[start_line] = true;
        self.comment[end_line] = true;
    }

    fn has_comment_or_empty(&self, line: usize) -> bool {
        self.blank.get(line).copied().unwrap_or(false)
            || self.comment.get(line).copied().unwrap_or(false)
    }

    fn line_start(&self, line: usize) -> usize {
        self.lines[line].start
    }

    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn is_same_line(&self, left_offset: usize, right_offset: usize) -> bool {
        self.line_index(left_offset) == self.line_index(right_offset)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CommentKind {
    Line,
    Block,
    Hashbang,
}

#[derive(Clone, Copy)]
struct Comment {
    kind: CommentKind,
    start: usize,
    end: usize,
    token_index: Option<usize>,
}

struct Options<'value> {
    value: &'value Value,
    before_block: bool,
    after_block: bool,
    before_line: bool,
    after_line: bool,
    after_hashbang: bool,
    apply_default_ignore_patterns: bool,
    custom_ignore: Option<Regex>,
}

impl<'value> Options<'value> {
    fn new(value: &'value Value) -> Self {
        let custom_ignore = first_option(value)
            .and_then(|option| option.get("ignorePattern"))
            .and_then(Value::as_str)
            .filter(|pattern| !pattern.is_empty())
            .and_then(|pattern| Regex::new(pattern).ok());
        Self {
            value,
            before_block: option_object_bool(value, "beforeBlockComment", true),
            after_block: option_object_bool(value, "afterBlockComment", false),
            before_line: option_object_bool(value, "beforeLineComment", false),
            after_line: option_object_bool(value, "afterLineComment", false),
            after_hashbang: option_object_bool(value, "afterHashbangComment", false),
            apply_default_ignore_patterns: option_object_bool(
                value,
                "applyDefaultIgnorePatterns",
                true,
            ),
            custom_ignore,
        }
    }

    fn boundary_allowed(&self, parent: NodeRecord, at_start: bool) -> bool {
        let suffix = if at_start { "Start" } else { "End" };
        let block_key = option_name("allowBlock", suffix);
        let class_key = option_name("allowClass", suffix);
        if option_object_bool(self.value, block_key, false)
            && parent.targets & BLOCK != 0
            && !(option_object(self.value, class_key) == Some(false) && parent.targets & CLASS != 0)
        {
            return true;
        }

        [
            ("allowClass", CLASS),
            ("allowObject", OBJECT),
            ("allowArray", ARRAY),
            ("allowInterface", INTERFACE),
            ("allowType", TYPE),
            ("allowEnum", ENUM),
            ("allowModule", MODULE),
        ]
        .into_iter()
        .any(|(prefix, target)| {
            let key = option_name(prefix, suffix);
            option_object_bool(self.value, key, false) && parent.targets & target != 0
        })
    }
}

fn option_name(prefix: &'static str, suffix: &'static str) -> &'static str {
    match (prefix, suffix) {
        ("allowBlock", "Start") => "allowBlockStart",
        ("allowBlock", "End") => "allowBlockEnd",
        ("allowClass", "Start") => "allowClassStart",
        ("allowClass", "End") => "allowClassEnd",
        ("allowObject", "Start") => "allowObjectStart",
        ("allowObject", "End") => "allowObjectEnd",
        ("allowArray", "Start") => "allowArrayStart",
        ("allowArray", "End") => "allowArrayEnd",
        ("allowInterface", "Start") => "allowInterfaceStart",
        ("allowInterface", "End") => "allowInterfaceEnd",
        ("allowType", "Start") => "allowTypeStart",
        ("allowType", "End") => "allowTypeEnd",
        ("allowEnum", "Start") => "allowEnumStart",
        ("allowEnum", "End") => "allowEnumEnd",
        ("allowModule", "Start") => "allowModuleStart",
        ("allowModule", "End") => "allowModuleEnd",
        _ => "",
    }
}

fn option_object(options: &Value, key: &str) -> Option<bool> {
    first_option(options)
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
}

pub(crate) fn check_lines_around_comment(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let normalized = Options::new(options);
    if !normalized.before_block
        && !normalized.after_block
        && !normalized.before_line
        && !normalized.after_line
        && !normalized.after_hashbang
    {
        return;
    }

    let mut lines = Lines::new(scan.source());
    let comments = comments(scan);
    for comment in &comments {
        lines.mark_comment(comment.start, comment.end);
    }
    let parents = collect_parent_nodes(scan);

    for comment in comments {
        let (before, after) = match comment.kind {
            CommentKind::Line => (normalized.before_line, normalized.after_line),
            CommentKind::Block => (normalized.before_block, normalized.after_block),
            CommentKind::Hashbang => (false, normalized.after_hashbang),
        };
        if (!before && !after)
            || ignored_comment(scan.source(), comment, &normalized)
            || code_around_comment(scan, &lines, comment)
        {
            continue;
        }

        let start_line = lines.line_index(comment.start);
        let end_line = lines.line_index(comment.end.saturating_sub(1).max(comment.start));
        let parent = parent_node(&parents, comment);
        let start_allowed = parent.is_some_and(|node| {
            normalized.boundary_allowed(node, true)
                && start_line
                    == lines
                        .line_index(node.boundary_start as usize)
                        .saturating_add(1)
        });
        let end_allowed = parent.is_some_and(|node| {
            normalized.boundary_allowed(node, false)
                && lines
                    .line_index(node.span.end.saturating_sub(1) as usize)
                    .checked_sub(end_line)
                    == Some(1)
        });

        if before
            && start_line > 0
            && !start_allowed
            && !lines.has_comment_or_empty(start_line - 1)
            && !adjacent_comment_on_same_line(scan, &lines, comment, true)
        {
            push_insert_line(
                diagnostics,
                "before",
                BEFORE_MESSAGE,
                comment,
                lines.line_start(start_line),
            );
        }

        if after
            && end_line + 1 < lines.line_count()
            && !end_allowed
            && !lines.has_comment_or_empty(end_line + 1)
            && !adjacent_comment_on_same_line(scan, &lines, comment, false)
        {
            push_insert_line(diagnostics, "after", AFTER_MESSAGE, comment, comment.end);
        }
    }
}

fn comments(scan: &Scan) -> Vec<Comment> {
    let mut comments = Vec::new();
    if scan.source().starts_with("#!") {
        let end = first_line_end(scan.source().as_bytes(), 2);
        comments.push(Comment {
            kind: CommentKind::Hashbang,
            start: 0,
            end,
            token_index: None,
        });
    }
    comments.extend(
        scan.tokens()
            .iter()
            .enumerate()
            .filter_map(|(index, token)| {
                let kind = match token.kind {
                    TokenKind::LineComment => CommentKind::Line,
                    TokenKind::BlockComment => CommentKind::Block,
                    _ => return None,
                };
                Some(Comment {
                    kind,
                    start: token.start,
                    end: token.end,
                    token_index: Some(index),
                })
            }),
    );
    comments
}

fn first_line_end(bytes: &[u8], mut cursor: usize) -> usize {
    while cursor < bytes.len() && newline_len_at(bytes, cursor) == 0 {
        cursor += 1;
    }
    cursor
}

fn ignored_comment(source: &str, comment: Comment, options: &Options<'_>) -> bool {
    if comment.kind == CommentKind::Hashbang {
        return false;
    }
    let value = comment_value(source, comment);
    (options.apply_default_ignore_patterns && matches_default_ignore_pattern(value))
        || options
            .custom_ignore
            .as_ref()
            .is_some_and(|pattern| pattern.is_match(value))
}

fn comment_value(source: &str, comment: Comment) -> &str {
    match comment.kind {
        CommentKind::Line => &source[comment.start + 2..comment.end],
        CommentKind::Block => {
            let value_end = if source[comment.start..comment.end].ends_with("*/") {
                comment.end - 2
            } else {
                comment.end
            };
            &source[comment.start + 2..value_end]
        }
        CommentKind::Hashbang => &source[comment.start + 2..comment.end],
    }
}

fn matches_default_ignore_pattern(value: &str) -> bool {
    let value = value.trim_start();
    value.starts_with("eslint")
        || value.starts_with("jscs")
        || starts_with_word_and_space(value, "jshint")
        || starts_with_word_and_space(value, "jslint")
        || starts_with_word_and_space(value, "istanbul")
        || starts_with_word_and_space(value, "global")
        || starts_with_word_and_space(value, "globals")
        || starts_with_word_and_space(value, "exported")
        || value
            .strip_prefix('/')
            .map(str::trim_start)
            .is_some_and(|rest| rest.starts_with("<reference") || rest.starts_with("<amd-"))
}

fn starts_with_word_and_space(value: &str, word: &str) -> bool {
    value
        .strip_prefix(word)
        .and_then(|rest| rest.chars().next())
        .is_some_and(char::is_whitespace)
}

fn code_around_comment(scan: &Scan, lines: &Lines, comment: Comment) -> bool {
    let Some(index) = comment.token_index else {
        return scan
            .tokens()
            .iter()
            .find(|token| token.start >= comment.end)
            .is_some_and(|token| lines.is_same_line(comment.end.saturating_sub(1), token.start));
    };
    let tokens = scan.tokens();
    let previous_code = (0..index)
        .rev()
        .find(|&candidate| !tokens[candidate].kind.is_comment());
    if previous_code.is_some_and(|candidate| {
        lines.is_same_line(tokens[candidate].end.saturating_sub(1), comment.start)
    }) {
        return true;
    }
    (index + 1..tokens.len())
        .find(|&candidate| !tokens[candidate].kind.is_comment())
        .is_some_and(|candidate| {
            lines.is_same_line(comment.end.saturating_sub(1), tokens[candidate].start)
        })
}

fn adjacent_comment_on_same_line(
    scan: &Scan,
    lines: &Lines,
    comment: Comment,
    before: bool,
) -> bool {
    let Some(index) = comment.token_index else {
        return false;
    };
    let candidate = if before {
        index.checked_sub(1)
    } else {
        index
            .checked_add(1)
            .filter(|next| *next < scan.tokens().len())
    };
    candidate.is_some_and(|candidate| {
        let token = &scan.tokens()[candidate];
        token.kind.is_comment()
            && if before {
                lines.is_same_line(token.end.saturating_sub(1), comment.start)
            } else {
                lines.is_same_line(comment.end.saturating_sub(1), token.start)
            }
    })
}

fn collect_parent_nodes(scan: &Scan) -> Vec<NodeRecord> {
    let allocator = Allocator::default();
    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        let parsed = Parser::new(&allocator, scan.source(), source_type).parse();
        if parsed.errors.is_empty() {
            let mut collector = ParentCollector { nodes: Vec::new() };
            collector.visit_program(&parsed.program);
            resolve_opening_braces(scan, &mut collector.nodes);
            return collector.nodes;
        }
    }
    Vec::new()
}

fn resolve_opening_braces(scan: &Scan, nodes: &mut [NodeRecord]) {
    for node in nodes {
        if node.targets & BLOCK == 0 {
            continue;
        }
        let needs_opening_brace =
            node.static_open.is_some() || node.boundary_start > node.span.start;
        if !needs_opening_brace {
            continue;
        }
        let search_start = node.boundary_start as usize;
        if let Some(token) = scan.tokens().iter().find(|token| {
            token.start >= search_start
                && token.end <= node.span.end as usize
                && token.kind == TokenKind::Punctuator
                && &scan.source()[token.start..token.end] == "{"
        }) {
            node.boundary_start = token.start as u32;
            if node.static_open.is_some() {
                node.static_open = Some(token.start as u32);
            }
        }
    }
}

fn parent_node(nodes: &[NodeRecord], comment: Comment) -> Option<NodeRecord> {
    let start = u32::try_from(comment.start).ok()?;
    let end = u32::try_from(comment.end).ok()?;
    let mut parent: Option<NodeRecord> = None;
    for node in nodes {
        if node.span.start <= start
            && node.span.end >= end
            && parent.is_none_or(|current| node.span.size() <= current.span.size())
        {
            parent = Some(*node);
        }
    }
    let parent = parent?;
    if parent.static_open.is_some_and(|open| start < open) {
        return None;
    }
    (parent.targets != 0).then_some(parent)
}

fn push_insert_line(
    diagnostics: &mut Vec<LintDiagnostic>,
    message_id: &'static str,
    message: &'static str,
    comment: Comment,
    insertion_offset: usize,
) {
    let (Ok(start), Ok(end), Ok(insertion)) = (
        u32::try_from(comment.start),
        u32::try_from(comment.end),
        u32::try_from(insertion_offset),
    ) else {
        return;
    };
    diagnostics.push(LintDiagnostic {
        rule_name: RULE_NAME.to_owned(),
        message_id: message_id.to_owned(),
        message: message.to_owned(),
        data: Default::default(),
        range: TextRange::new(start, end),
        suggestions: std::iter::once(LintSuggestion {
            message_id: "insertLine".to_owned(),
            message: INSERT_LINE_MESSAGE.to_owned(),
            fixes: std::iter::once(LintFix::replace_range(
                TextRange::new(insertion, insertion),
                "\n",
            ))
            .collect(),
        })
        .collect(),
    });
}

fn newline_len_at(bytes: &[u8], index: usize) -> usize {
    match bytes.get(index..) {
        Some([b'\r', b'\n', ..]) => 2,
        Some([b'\r' | b'\n', ..]) => 1,
        Some([0xe2, 0x80, 0xa8 | 0xa9, ..]) => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn diagnostics(source: &str, options: &str) -> Vec<LintDiagnostic> {
        let value: Value = serde_json::from_str(options).expect("valid test options");
        diagnostics_for_value(source, &value)
    }

    fn diagnostics_for_value(source: &str, options: &Value) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check_lines_around_comment(&scan, options, &mut diagnostics);
        diagnostics
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> String {
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        fixes.sort_by_key(|fix| std::cmp::Reverse(fix.range.start));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        output
    }

    fn byte_offset_for_location(source: &str, line: usize, column: usize) -> usize {
        let lines = Lines::new(source);
        let line_start = lines.lines[line - 1].start;
        let mut utf16_column = 1;
        for (relative_offset, character) in source[line_start..].char_indices() {
            if utf16_column == column {
                return line_start + relative_offset;
            }
            utf16_column += character.len_utf16();
        }
        source.len()
    }

    fn byte_offset_for_utf16(source: &str, target: usize) -> usize {
        let mut utf16_offset = 0;
        for (byte_offset, character) in source.char_indices() {
            if utf16_offset == target {
                return byte_offset;
            }
            utf16_offset += character.len_utf16();
        }
        source.len()
    }

    #[test]
    fn defaults_to_before_block_comments() {
        let source = "bar();\n/** docs */\nconst value = 1;";
        let found = diagnostics(source, "[]");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].message_id, "before");
        assert_eq!(
            apply_fixes(source, &found),
            "bar();\n\n/** docs */\nconst value = 1;"
        );
    }

    #[test]
    fn recognizes_all_ecmascript_line_terminators() {
        for separator in ["\r\n", "\r", "\n", "\u{2028}", "\u{2029}"] {
            let source = ["before();", separator, "// note", separator, "after();"].concat();
            let found = diagnostics(
                &source,
                r#"[{"beforeLineComment":true,"afterLineComment":true}]"#,
            );
            assert_eq!(found.len(), 2, "{separator:?}");
            assert_eq!(
                found
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                ["before", "after"],
                "{separator:?}"
            );
            let comment_start = "before();".len() + separator.len();
            let comment_end = comment_start + "// note".len();
            assert_eq!(
                found
                    .iter()
                    .map(|diagnostic| diagnostic.range)
                    .collect::<Vec<_>>(),
                [
                    TextRange::new(comment_start as u32, comment_end as u32),
                    TextRange::new(comment_start as u32, comment_end as u32),
                ],
                "{separator:?}"
            );
            assert_eq!(
                found
                    .iter()
                    .map(|diagnostic| diagnostic.suggestions[0].fixes[0].range)
                    .collect::<Vec<_>>(),
                [
                    TextRange::new(comment_start as u32, comment_start as u32),
                    TextRange::new(comment_end as u32, comment_end as u32),
                ],
                "{separator:?}"
            );
            assert_eq!(
                apply_fixes(&source, &found),
                ["before();", separator, "\n// note\n", separator, "after();"].concat(),
                "{separator:?}"
            );
        }
    }

    #[test]
    fn preserves_unicode_byte_ranges_and_fix_offsets() {
        let source = "const 絵 = 1;\n// 注釈\nconst 後 = 2;";
        let found = diagnostics(
            source,
            r#"[{"beforeLineComment":true,"afterLineComment":true}]"#,
        );
        assert_eq!(found.len(), 2);
        let start = source.find("// 注釈").expect("comment");
        let end = start + "// 注釈".len();
        assert!(
            found
                .iter()
                .all(|diagnostic| { diagnostic.range == TextRange::new(start as u32, end as u32) })
        );
        assert_eq!(
            apply_fixes(source, &found),
            "const 絵 = 1;\n\n// 注釈\n\nconst 後 = 2;"
        );
    }

    #[test]
    fn ignores_directives_and_custom_patterns() {
        for comment in [
            "/* eslint-disable */",
            "/* jshint strict */",
            "/* global value */",
            "/// <reference path=\"types.d.ts\" />",
        ] {
            let source = ["before();\n", comment, "\nafter();"].concat();
            assert!(diagnostics(&source, "[]").is_empty(), "{comment}");
        }
        assert!(
            diagnostics(
                "before();\n/** generated docs */\nafter();",
                r#"[{"ignorePattern":"generated"}]"#
            )
            .is_empty()
        );
    }

    #[test]
    fn handles_hashbang_independently_from_line_comments() {
        let source = "#!/usr/bin/env node\nconst value = 1;";
        let found = diagnostics(source, r#"[{"afterHashbangComment":true}]"#);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].message_id, "after");
        assert_eq!(
            apply_fixes(source, &found),
            "#!/usr/bin/env node\n\nconst value = 1;"
        );
    }

    #[test]
    fn never_reports_inline_or_comment_cluster_false_positives() {
        for source in [
            "before(); // inline\nafter();",
            "before();\n/* one */ /* two */\nafter();",
            "before();\n// one\n// two\nafter();",
        ] {
            let found = diagnostics(
                source,
                r#"[{"beforeLineComment":true,"afterLineComment":true,"beforeBlockComment":true,"afterBlockComment":true}]"#,
            );
            if source.contains("inline") {
                assert!(found.is_empty(), "{source}");
            } else {
                assert_eq!(found.len(), 2, "{source}");
            }
        }
    }

    #[test]
    fn replays_every_stable_v5_10_fixture_exactly() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/lines-around-comment-v5.10.0.json"
        ))
        .expect("valid pinned upstream fixture");
        assert_eq!(
            fixture["upstream"].as_str(),
            Some("@stylistic/eslint-plugin@5.10.0")
        );
        let suites = fixture["suites"].as_array().expect("fixture suites");
        assert_eq!(suites.len(), 2);

        let mut valid_count = 0;
        let mut invalid_count = 0;
        let mut error_count = 0;
        for suite in suites {
            for test in suite["valid"].as_array().expect("valid fixtures") {
                valid_count += 1;
                let source = test["code"].as_str().expect("valid source");
                let found = diagnostics_for_value(source, &test["options"]);
                assert!(found.is_empty());
            }

            for test in suite["invalid"].as_array().expect("invalid fixtures") {
                invalid_count += 1;
                let source = test["code"].as_str().expect("invalid source");
                let expected_errors = test["errors"].as_array().expect("expected errors");
                error_count += expected_errors.len();
                let found = diagnostics_for_value(source, &test["options"]);
                assert_eq!(found.len(), expected_errors.len());

                for (diagnostic, expected) in found.iter().zip(expected_errors) {
                    assert_eq!(
                        diagnostic.message_id,
                        expected["messageId"].as_str().expect("message id")
                    );
                    assert_eq!(
                        diagnostic.message,
                        expected["message"].as_str().expect("message")
                    );

                    let report_range = expected["reportRange"].as_array().expect("report range");
                    let line = report_range[0].as_u64().expect("start line") as usize;
                    let column = report_range[1].as_u64().expect("start column") as usize;
                    let end_line = report_range[2].as_u64().expect("end line") as usize;
                    let end_column = report_range[3].as_u64().expect("end column") as usize;
                    assert_eq!(
                        diagnostic.range,
                        TextRange::new(
                            byte_offset_for_location(source, line, column) as u32,
                            byte_offset_for_location(source, end_line, end_column) as u32,
                        )
                    );

                    let expected_fix = &expected["fix"];
                    let fix_range = expected_fix["range"].as_array().expect("fix range");
                    let start = fix_range[0].as_u64().expect("fix start") as usize;
                    let end = fix_range[1].as_u64().expect("fix end") as usize;
                    assert_eq!(diagnostic.suggestions.len(), 1);
                    assert_eq!(diagnostic.suggestions[0].fixes.len(), 1);
                    assert_eq!(
                        diagnostic.suggestions[0].fixes[0],
                        LintFix::replace_range(
                            TextRange::new(
                                byte_offset_for_utf16(source, start) as u32,
                                byte_offset_for_utf16(source, end) as u32,
                            ),
                            expected_fix["text"].as_str().expect("fix text"),
                        )
                    );
                }

                assert_eq!(
                    apply_fixes(source, &found),
                    test["output"].as_str().expect("fixed output")
                );
            }
        }

        assert_eq!(valid_count, 157);
        assert_eq!(invalid_count, 104);
        assert_eq!(error_count, 113);
    }
}
