//! Top-level scan loop: extracts import/export chunks and drives the sort.
//!
//! Mirrors `imports.js` and `exports.js` from eslint-plugin-simple-import-sort.
//! Each public function accepts `all_comments: &[Comment]` (pre-collected from
//! `program.comments` in source order) instead of calling ESLint's
//! `sourceCode.getCommentsBefore/After`.

use oxc_ast::ast::{
    Comment, CommentKind, ExportAllDeclaration, ExportNamedDeclaration, ImportDeclaration,
    ImportDeclarationSpecifier, ImportOrExportKind, ModuleExportName, Statement,
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::shared::{
    self, SIDE_EFFECT_STYLE, SourceInfo, compare, get_indentation, get_trailing_spaces,
    guess_newline, import_style, print_with_sorted_specifiers,
};
use crate::types::{Diagnostic, DiagnosticFix, LineIndex, RuleKind, SimpleImportSortOptions};

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

pub(crate) fn scan_import_chunks(
    source_text: &str,
    line_index: &LineIndex,
    statements: &[Statement<'_>],
    all_comments: &[Comment],
    options: &SimpleImportSortOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    // extractChunks: ImportDeclaration → PartOfChunk, else NotPartOfChunk
    let mut chunk: SmallVec<[&ImportDeclaration<'_>; 16]> = SmallVec::new();
    for statement in statements {
        if let Statement::ImportDeclaration(decl) = statement {
            chunk.push(decl);
        } else {
            report_import_chunk(
                source_text,
                line_index,
                all_comments,
                &chunk,
                options,
                diagnostics,
            );
            chunk.clear();
        }
    }
    report_import_chunk(
        source_text,
        line_index,
        all_comments,
        &chunk,
        options,
        diagnostics,
    );
}

pub(crate) fn scan_export_chunks(
    source_text: &str,
    line_index: &LineIndex,
    statements: &[Statement<'_>],
    all_comments: &[Comment],
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    // extractChunks with isPartOfChunk from exports.js
    let mut chunk: SmallVec<[ExportNode<'_>; 16]> = SmallVec::new();

    for statement in statements {
        match statement {
            Statement::ExportNamedDeclaration(decl) if is_export_from_named(decl) => {
                // isPartOfChunk: check for grouping comment
                let last_end = chunk.last().map(|n| n.span().end);
                let part =
                    export_is_part_of_chunk(source_text, all_comments, decl.span.start, last_end);
                if part == ChunkPart::NewChunk {
                    report_export_chunk(source_text, line_index, all_comments, &chunk, diagnostics);
                    chunk.clear();
                }
                chunk.push(ExportNode::Named(decl));
            }
            Statement::ExportAllDeclaration(decl) => {
                let last_end = chunk.last().map(|n| n.span().end);
                let part =
                    export_is_part_of_chunk(source_text, all_comments, decl.span.start, last_end);
                if part == ChunkPart::NewChunk {
                    report_export_chunk(source_text, line_index, all_comments, &chunk, diagnostics);
                    chunk.clear();
                }
                chunk.push(ExportNode::All(decl));
            }
            // Local export { a, b } (no source, no declaration)
            Statement::ExportNamedDeclaration(decl)
                if decl.source.is_none() && decl.declaration.is_none() =>
            {
                report_export_chunk(source_text, line_index, all_comments, &chunk, diagnostics);
                chunk.clear();
                // Sort specifiers in place if >1
                if decl.specifiers.len() > 1 {
                    report_local_export_specifiers(
                        source_text,
                        line_index,
                        all_comments,
                        decl,
                        diagnostics,
                    );
                }
            }
            _ => {
                report_export_chunk(source_text, line_index, all_comments, &chunk, diagnostics);
                chunk.clear();
            }
        }
    }
    report_export_chunk(source_text, line_index, all_comments, &chunk, diagnostics);
}

// ---------------------------------------------------------------------------
// ExportNode – unifies ExportNamedDeclaration and ExportAllDeclaration
// ---------------------------------------------------------------------------

enum ExportNode<'a> {
    Named(&'a ExportNamedDeclaration<'a>),
    All(&'a ExportAllDeclaration<'a>),
}

impl<'a> ExportNode<'a> {
    fn span(&self) -> Span {
        match self {
            ExportNode::Named(d) => d.span,
            ExportNode::All(d) => d.span,
        }
    }
    fn source_str(&self) -> &str {
        match self {
            ExportNode::Named(d) => d.source.as_ref().expect("export-from").value.as_str(),
            ExportNode::All(d) => d.source.value.as_str(),
        }
    }
    fn export_kind(&self) -> ImportOrExportKind {
        match self {
            ExportNode::Named(d) => d.export_kind,
            ExportNode::All(d) => d.export_kind,
        }
    }
    fn specifier_spans(&self) -> SmallVec<[Span; 8]> {
        match self {
            ExportNode::Named(d) => d.specifiers.iter().map(|s| s.span).collect(),
            ExportNode::All(_) => SmallVec::new(),
        }
    }
    fn specifier_sort_keys(&self) -> SmallVec<[(CompactString, CompactString, u8); 8]> {
        match self {
            ExportNode::Named(d) => d
                .specifiers
                .iter()
                .map(|s| {
                    (
                        module_export_name(&s.exported),
                        module_export_name(&s.local),
                        kind_rank(s.export_kind),
                    )
                })
                .collect(),
            ExportNode::All(_) => SmallVec::new(),
        }
    }
}

fn is_export_from_named(decl: &ExportNamedDeclaration<'_>) -> bool {
    decl.source.is_some() && decl.declaration.is_none()
}

// ---------------------------------------------------------------------------
// isPartOfChunk for exports (exports.js)
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
enum ChunkPart {
    Part,
    NewChunk,
}

/// Mirrors `isPartOfChunk(node, lastNode, sourceCode)` in exports.js.
/// Returns NewChunk if there's a "grouping comment" before this node.
fn export_is_part_of_chunk(
    source_text: &str,
    all_comments: &[Comment],
    node_start: u32,
    last_node_end: Option<u32>,
) -> ChunkPart {
    let node_start_line = shared::line_of(source_text, node_start);
    let last_end_line = last_node_end.map(|e| shared::line_of(source_text, e));

    for comment in all_comments {
        // Only consider comments that start before the node
        if comment.span.start >= node_start {
            break;
        }
        // Mirrors upstream `isPartOfChunk` filter in exports.js:
        //   (lastNode == null || comment.loc.start.line > lastNode.loc.end.line)
        //   && comment.loc.end.line < node.loc.start.line
        let c_start_line = shared::line_of(source_text, comment.span.start);
        let c_end_line = shared::line_of(source_text, comment.span.end);

        let after_last = match last_end_line {
            None => true,
            Some(le) => c_start_line > le,
        };
        if after_last && c_end_line < node_start_line {
            return ChunkPart::NewChunk;
        }
    }
    ChunkPart::Part
}

// ---------------------------------------------------------------------------
// ImportExportItem – one import/export statement with full code + sort keys
// ---------------------------------------------------------------------------

struct ImportExportItem {
    node_end: u32,
    /// Full text: indentation + commentsBefore + sortedNode + commentsAfter + trailingSpaces
    code: CompactString,
    /// Extended span for the chunk range comparison
    start: u32,
    end: u32,
    source: SourceInfo,
    style: u8,
    index: usize,
    needs_newline: bool,
    outer_group: usize,
    inner_group: usize,
}

// ---------------------------------------------------------------------------
// report_import_chunk (imports.js maybeReportChunkSorting)
// ---------------------------------------------------------------------------

fn report_import_chunk(
    source_text: &str,
    line_index: &LineIndex,
    all_comments: &[Comment],
    chunk: &[&ImportDeclaration<'_>],
    options: &SimpleImportSortOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    if chunk.is_empty() {
        return;
    }
    let newline = guess_newline(source_text);

    // handleLastSemicolon
    let last_node_end = handle_last_semicolon(
        source_text,
        chunk.last().expect("non-empty").span,
        all_comments,
    );

    let items = build_import_items(
        source_text,
        all_comments,
        chunk,
        last_node_end,
        options,
        newline,
    );

    let sorted_items = make_sorted_import_items(&items, options);
    let sorted = print_sorted_items(&sorted_items, &items, source_text, all_comments, newline);

    let start = items[0].start;
    let end = items[items.len() - 1].end;
    maybe_report_sorting(
        source_text,
        line_index,
        sorted,
        start,
        end,
        RuleKind::Imports,
        diagnostics,
    );
}

fn build_import_items(
    source_text: &str,
    all_comments: &[Comment],
    chunk: &[&ImportDeclaration<'_>],
    last_node_end: u32,
    options: &SimpleImportSortOptions,
    newline: &str,
) -> SmallVec<[ImportExportItem; 16]> {
    let chunk_len = chunk.len();
    let mut items: SmallVec<[ImportExportItem; 16]> = SmallVec::new();

    for (node_index, decl) in chunk.iter().enumerate() {
        let node_start = decl.span.start;
        let node_end = if node_index == chunk_len - 1 {
            last_node_end
        } else {
            decl.span.end
        };

        // last_line for commentsBefore filter
        let last_line = if node_index == 0 {
            shared::line_of(source_text, node_start).saturating_sub(1)
        } else {
            shared::line_of(source_text, chunk[node_index - 1].span.end)
        };
        let node_start_line = shared::line_of(source_text, node_start);
        let node_end_line = shared::line_of(source_text, node_end);

        let comments_before = shared::comments_before_node(
            all_comments,
            source_text,
            node_start,
            node_start_line,
            last_line,
            node_index == 0,
        );
        let comments_after =
            shared::comments_after_node(all_comments, source_text, node_end, node_end_line);

        // specifier spans and sort keys (only ImportSpecifier, not default/namespace)
        let spec_spans: SmallVec<[Span; 8]> = decl
            .specifiers
            .as_ref()
            .map(|specs| {
                specs
                    .iter()
                    .filter_map(|s| {
                        if let ImportDeclarationSpecifier::ImportSpecifier(is) = s {
                            Some(is.span)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let spec_keys: SmallVec<[(CompactString, CompactString, u8); 8]> = decl
            .specifiers
            .as_ref()
            .map(|specs| {
                specs
                    .iter()
                    .filter_map(|s| {
                        if let ImportDeclarationSpecifier::ImportSpecifier(is) = s {
                            Some((
                                module_export_name(&is.imported),
                                CompactString::from(is.local.name.as_str()),
                                kind_rank(is.import_kind),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let style = import_style(decl, source_text);
        let kind_str = import_kind_str(decl.import_kind);
        let orig = decl.source.value.as_str();
        let source = shared::get_source(orig, kind_str);
        let (outer_group, inner_group) = import_group(style, decl.import_kind, orig, options);

        let item = build_item(
            source_text,
            all_comments,
            node_start,
            node_end,
            node_end_line,
            &comments_before,
            &comments_after,
            node_index,
            &spec_spans,
            &spec_keys,
            newline,
            style,
            source,
            outer_group,
            inner_group,
        );
        items.push(item);
    }
    items
}

// ---------------------------------------------------------------------------
// report_export_chunk (exports.js maybeReportChunkSorting)
// ---------------------------------------------------------------------------

fn report_export_chunk(
    source_text: &str,
    line_index: &LineIndex,
    all_comments: &[Comment],
    chunk: &[ExportNode<'_>],
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    if chunk.is_empty() {
        return;
    }
    let newline = guess_newline(source_text);

    let last_node_end =
        handle_last_semicolon(source_text, chunk[chunk.len() - 1].span(), all_comments);

    let items = build_export_items(source_text, all_comments, chunk, last_node_end, newline);

    // sortImportExportItems – single group for exports
    let sorted_refs = sort_import_export_items(items.iter().collect());
    // Wrap in the nested structure expected by print_sorted_items
    let single_group: SmallVec<[&ImportExportItem; 8]> = sorted_refs;
    let inner: SmallVec<[SmallVec<[&ImportExportItem; 8]>; 4]> = {
        let mut v = SmallVec::new();
        v.push(single_group);
        v
    };
    let outer: OuterGroups<'_> = {
        let mut v = SmallVec::new();
        v.push(inner);
        v
    };

    let sorted = print_sorted_items(&outer, &items, source_text, all_comments, newline);

    let start = items[0].start;
    let end = items[items.len() - 1].end;
    maybe_report_sorting(
        source_text,
        line_index,
        sorted,
        start,
        end,
        RuleKind::Exports,
        diagnostics,
    );
}

fn build_export_items(
    source_text: &str,
    all_comments: &[Comment],
    chunk: &[ExportNode<'_>],
    last_node_end: u32,
    newline: &str,
) -> SmallVec<[ImportExportItem; 16]> {
    let chunk_len = chunk.len();
    let mut items: SmallVec<[ImportExportItem; 16]> = SmallVec::new();

    for (node_index, node) in chunk.iter().enumerate() {
        let node_start = node.span().start;
        let node_end = if node_index == chunk_len - 1 {
            last_node_end
        } else {
            node.span().end
        };

        let last_line = if node_index == 0 {
            shared::line_of(source_text, node_start).saturating_sub(1)
        } else {
            shared::line_of(source_text, chunk[node_index - 1].span().end)
        };
        let node_start_line = shared::line_of(source_text, node_start);
        let node_end_line = shared::line_of(source_text, node_end);

        let comments_before = shared::comments_before_node(
            all_comments,
            source_text,
            node_start,
            node_start_line,
            last_line,
            node_index == 0,
        );
        let comments_after =
            shared::comments_after_node(all_comments, source_text, node_end, node_end_line);

        let orig = node.source_str();
        let kind_str = import_kind_str(node.export_kind());
        let source = shared::get_source(orig, kind_str);
        let spec_spans = node.specifier_spans();
        let spec_keys = node.specifier_sort_keys();

        let item = build_item(
            source_text,
            all_comments,
            node_start,
            node_end,
            node_end_line,
            &comments_before,
            &comments_after,
            node_index,
            &spec_spans,
            &spec_keys,
            newline,
            1u8, // getStyle: always 1 for exports
            source,
            0, // outer_group
            0, // inner_group
        );
        items.push(item);
    }
    items
}

// ---------------------------------------------------------------------------
// report_local_export_specifiers (exports.js maybeReportExportSpecifierSorting)
// ---------------------------------------------------------------------------

fn report_local_export_specifiers(
    source_text: &str,
    line_index: &LineIndex,
    all_comments: &[Comment],
    decl: &ExportNamedDeclaration<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    let spec_spans: SmallVec<[Span; 8]> = decl.specifiers.iter().map(|s| s.span).collect();
    let sort_keys: SmallVec<[(CompactString, CompactString, u8); 8]> = decl
        .specifiers
        .iter()
        .map(|s| {
            (
                module_export_name(&s.exported),
                module_export_name(&s.local),
                kind_rank(s.export_kind),
            )
        })
        .collect();

    let newline = guess_newline(source_text);
    let sorted = print_with_sorted_specifiers(
        source_text,
        decl.span.start,
        decl.span.end,
        all_comments,
        &spec_spans,
        &sort_keys,
        newline,
    );

    let start = decl.span.start;
    let end = decl.span.end;
    maybe_report_sorting(
        source_text,
        line_index,
        sorted,
        start,
        end,
        RuleKind::Exports,
        diagnostics,
    );
}

// ---------------------------------------------------------------------------
// Core item builder – mirrors the closure inside getImportExportItems.map
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn build_item(
    source_text: &str,
    all_comments: &[Comment],
    node_start: u32,
    node_end: u32,
    node_end_line: u32,
    comments_before: &[&Comment],
    comments_after: &[&Comment],
    node_index: usize,
    spec_spans: &[Span],
    spec_keys: &[(CompactString, CompactString, u8)],
    newline: &str,
    style: u8,
    source: SourceInfo,
    outer_group: usize,
    inner_group: usize,
) -> ImportExportItem {
    // printCommentsBefore
    let before = shared::print_comments_before(source_text, node_start, comments_before);
    // printCommentsAfter
    let after = shared::print_comments_after(source_text, node_end, comments_after);

    // indentation: from the "first" token (first comment or node)
    let first_start = comments_before
        .first()
        .map(|c| c.span.start)
        .unwrap_or(node_start);
    // getIndentation: find the token before first_start
    let token_before_end = find_last_token_end_before(source_text, all_comments, first_start);
    let indentation = get_indentation(source_text, token_before_end, first_start);

    // trailingSpaces: from last token (last comment or node)
    let last_end = comments_after
        .last()
        .map(|c| c.span.end)
        .unwrap_or(node_end);
    // getTrailingSpaces: find the token after last_end
    let next_token_start = find_first_token_start_after(source_text, all_comments, last_end);
    let trailing_spaces = get_trailing_spaces(source_text, last_end, next_token_start);

    // printWithSortedSpecifiers: reorder specifiers inside { }
    let node_with_sorted = print_with_sorted_specifiers(
        source_text,
        node_start,
        node_end,
        all_comments,
        spec_spans,
        spec_keys,
        newline,
    );

    // code = indentation + before + nodeWithSorted + after + trailingSpaces
    let mut code = CompactString::new("");
    code.push_str(&indentation);
    code.push_str(&before);
    code.push_str(&node_with_sorted);
    code.push_str(&after);
    code.push_str(&trailing_spaces);

    // Extended range
    let all_start = comments_before
        .first()
        .map(|c| c.span.start)
        .unwrap_or(node_start);
    let all_end = comments_after
        .last()
        .map(|c| c.span.end)
        .unwrap_or(node_end);

    let extended_start = all_start.saturating_sub(indentation.len() as u32);
    let extended_end = all_end + trailing_spaces.len() as u32;

    let needs_newline = comments_after
        .last()
        .is_some_and(|c| c.kind == CommentKind::Line);

    let _ = node_end_line; // used by caller for comments_after_node, not here

    ImportExportItem {
        node_end,
        code,
        start: extended_start,
        end: extended_end,
        source,
        style,
        index: node_index,
        needs_newline,
        outer_group,
        inner_group,
    }
}

/// Find the end position of the last "token" (comment or non-whitespace content)
/// strictly before `offset`. Used as "tokenBefore" in `getIndentation`.
///
/// We can't walk the full AST, so we approximate:
/// 1. Check for the last comment ending at or before `offset`.
/// 2. If there's non-whitespace source text before `offset` (past any comments),
///    scan backwards to find the end of the last non-whitespace character.
fn find_last_token_end_before(
    source_text: &str,
    all_comments: &[Comment],
    offset: u32,
) -> Option<u32> {
    // Find last comment that ends at or before offset
    let last_comment_end: Option<u32> = all_comments
        .iter()
        .rfind(|c| c.span.end <= offset)
        .map(|c| c.span.end);

    // Check if there's non-whitespace text between last_comment_end and offset
    // (or from the start of file if no comment)
    let check_from = last_comment_end.unwrap_or(0);
    let prefix = source_text
        .get(check_from as usize..offset as usize)
        .unwrap_or("");

    if prefix.trim_matches(|c: char| c.is_whitespace()).is_empty() {
        // No non-whitespace between last_comment and offset
        // If there's a comment before, use it as tokenBefore
        return last_comment_end;
    }

    // There's non-whitespace source content before offset (the previous AST node).
    // Scan backwards from offset to find the end of the last non-whitespace char.
    // We look at prefix (from check_from to offset).
    let trimmed = prefix.trim_end_matches(|c: char| c.is_whitespace());
    let token_end = check_from + trimmed.len() as u32;
    Some(token_end)
}

/// Find the start of the first comment or non-whitespace token at or after `offset`.
fn find_first_token_start_after(
    source_text: &str,
    all_comments: &[Comment],
    offset: u32,
) -> Option<u32> {
    // Find first comment at or after offset
    for c in all_comments {
        if c.span.start >= offset {
            return Some(c.span.start);
        }
    }
    // Otherwise find first non-whitespace
    let after = source_text.get(offset as usize..)?;
    if after.trim_matches(|c: char| c.is_whitespace()).is_empty() {
        None
    } else {
        let skip = after.find(|c: char| !c.is_whitespace())?;
        Some(offset + skip as u32)
    }
}

// ---------------------------------------------------------------------------
// handleLastSemicolon (shared.js)
// ---------------------------------------------------------------------------

/// Mirrors `handleLastSemicolon(chunk, sourceCode)`.
///
/// If the last token of `node_span` is `;` and it's NOT on the same line as
/// the next-to-last token, AND there's code after the `;`, adjust the node end
/// to the next-to-last token's end (i.e. the end of the `from "..."` string).
fn handle_last_semicolon(source_text: &str, node_span: Span, all_comments: &[Comment]) -> u32 {
    let text = source_text
        .get(node_span.start as usize..node_span.end as usize)
        .unwrap_or("");

    // Find last non-whitespace char in node text
    let last_byte_offset = {
        let trimmed = text.trim_end_matches(|c: char| c.is_whitespace());
        trimmed.len()
    };
    if last_byte_offset == 0 {
        return node_span.end;
    }

    let abs_last = node_span.start + last_byte_offset as u32;
    let last_char = source_text
        .get(abs_last as usize - 1..abs_last as usize)
        .unwrap_or("");

    if last_char != ";" {
        return node_span.end;
    }

    let semi_start = abs_last - 1; // ';' is 1 byte
    let semi_line = shared::line_of(source_text, semi_start);

    // Find next-to-last token (everything in node before the `;`)
    let next_to_last_end = {
        let before_semi = source_text
            .get(node_span.start as usize..semi_start as usize)
            .unwrap_or("");
        let trimmed = before_semi.trim_end_matches(|c: char| c.is_whitespace());
        node_span.start + trimmed.len() as u32
    };
    let ntl_line = shared::line_of(source_text, next_to_last_end.saturating_sub(1));

    if ntl_line == semi_line {
        // Same line → semicolon belongs to this node
        return node_span.end;
    }

    // Check if there's code after the `;` (i.e., `getTokenAfter(lastToken) != null`)
    let after_semi_start = abs_last;
    let has_code = has_code_after(source_text, after_semi_start, all_comments);
    if !has_code {
        // No code after → semicolon belongs to node (EOF case)
        return node_span.end;
    }

    // Semicolon belongs to next statement
    next_to_last_end
}

/// Returns true if there's a non-whitespace token at or after `offset`.
fn has_code_after(source_text: &str, offset: u32, all_comments: &[Comment]) -> bool {
    let mut cursor = offset;
    for c in all_comments {
        if c.span.start < offset {
            continue;
        }
        let gap = source_text
            .get(cursor as usize..c.span.start as usize)
            .unwrap_or("");
        if !gap.trim_matches(|ch: char| ch.is_whitespace()).is_empty() {
            return true;
        }
        cursor = c.span.end;
    }
    let rest = source_text.get(cursor as usize..).unwrap_or("");
    !rest.trim_matches(|ch: char| ch.is_whitespace()).is_empty()
}

// ---------------------------------------------------------------------------
// Import grouping (mirrors imports.js makeSortedItems / group matching)
// ---------------------------------------------------------------------------

fn import_group(
    style: u8,
    import_kind: ImportOrExportKind,
    original: &str,
    options: &SimpleImportSortOptions,
) -> (usize, usize) {
    let kind_rank_val = kind_rank(import_kind);
    if options.import_groups.is_empty() {
        // Default 5 groups
        if style == SIDE_EFFECT_STYLE {
            return (0, 0);
        }
        if original.starts_with("node:") {
            return (1, 0);
        }
        if is_package_source(original) {
            return (2, 0);
        }
        if original.starts_with('.') {
            return (4, 0);
        }
        return (3, 0);
    }
    // Custom groups: find longest match
    let match_source = import_match_source(style, kind_rank_val, original);
    let mut best: Option<(usize, usize, usize)> = None;
    for (outer_index, group) in options.import_groups.iter().enumerate() {
        for (inner_index, pattern) in group.iter().enumerate() {
            let Ok(re) = regex::Regex::new(pattern.as_str()) else {
                continue;
            };
            let Some(m) = re.find(match_source.as_str()) else {
                continue;
            };
            let len = m.end() - m.start();
            if best.is_none_or(|(_, _, bl)| len > bl) {
                best = Some((outer_index, inner_index, len));
            }
        }
    }
    best.map(|(o, i, _)| (o, i))
        .unwrap_or((options.import_groups.len(), 0))
}

fn import_match_source(style: u8, kind_rank: u8, original: &str) -> CompactString {
    let mut s = CompactString::new("");
    if style == SIDE_EFFECT_STYLE {
        s.push('\0');
    }
    s.push_str(original);
    if kind_rank == 0 {
        s.push('\0');
    }
    s
}

fn is_package_source(s: &str) -> bool {
    // Mirrors the default group regex `^@?\w`:
    // - starts with `@` followed by a word char (`[a-zA-Z0-9_]`), OR
    // - starts with a word char directly
    // `\w` (even under the `u` flag) is ASCII-only: [A-Za-z0-9_].
    let mut chars = s.chars();
    match chars.next() {
        None => false,
        Some('@') => chars
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_'),
        Some(c) => c.is_ascii_alphanumeric() || c == '_',
    }
}

// ---------------------------------------------------------------------------
// makeSortedItems (imports.js)
// ---------------------------------------------------------------------------

type OuterGroups<'a> = SmallVec<[SmallVec<[SmallVec<[&'a ImportExportItem; 8]>; 4]>; 8]>;

/// Mirrors `makeSortedItems(items, outerGroups)` from imports.js.
fn make_sorted_import_items<'a>(
    items: &'a [ImportExportItem],
    options: &SimpleImportSortOptions,
) -> OuterGroups<'a> {
    let n_outer_groups = if options.import_groups.is_empty() {
        5
    } else {
        options.import_groups.len()
    };
    let inner_counts: SmallVec<[usize; 8]> = if options.import_groups.is_empty() {
        (0..n_outer_groups).map(|_| 1).collect()
    } else {
        options
            .import_groups
            .iter()
            .map(|g| g.len().max(1))
            .collect()
    };

    // Build bucket grid: [outer][inner] → vec of items
    let mut buckets: OuterGroups<'_> = SmallVec::new();
    for &ni in &inner_counts {
        let mut outer: SmallVec<[SmallVec<[&ImportExportItem; 8]>; 4]> = SmallVec::new();
        for _ in 0..ni {
            outer.push(SmallVec::new());
        }
        buckets.push(outer);
    }
    let mut rest: SmallVec<[&ImportExportItem; 8]> = SmallVec::new();

    for item in items {
        let (o, i) = (item.outer_group, item.inner_group);
        if o < buckets.len() && i < buckets[o].len() {
            buckets[o][i].push(item);
        } else {
            rest.push(item);
        }
    }

    // Append rest group, filter empty, sort each group
    let mut all_outer = buckets;
    let mut rest_outer: SmallVec<[SmallVec<[&ImportExportItem; 8]>; 4]> = SmallVec::new();
    rest_outer.push(rest);
    all_outer.push(rest_outer);

    all_outer
        .into_iter()
        .map(|outer_groups| {
            let non_empty: SmallVec<[SmallVec<[&ImportExportItem; 8]>; 4]> =
                outer_groups.into_iter().filter(|g| !g.is_empty()).collect();
            non_empty
                .into_iter()
                .map(sort_import_export_items)
                .collect()
        })
        .filter(|groups: &SmallVec<[SmallVec<[&ImportExportItem; 8]>; 4]>| !groups.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// sortImportExportItems (shared.js)
// ---------------------------------------------------------------------------

fn sort_import_export_items(
    mut refs: SmallVec<[&ImportExportItem; 8]>,
) -> SmallVec<[&ImportExportItem; 8]> {
    refs.sort_by(|a, b| {
        // Side-effects: keep relative order, sort first
        if a.style == SIDE_EFFECT_STYLE && b.style == SIDE_EFFECT_STYLE {
            return a.index.cmp(&b.index);
        }
        if a.style == SIDE_EFFECT_STYLE {
            return std::cmp::Ordering::Less;
        }
        if b.style == SIDE_EFFECT_STYLE {
            return std::cmp::Ordering::Greater;
        }
        // Compare source key
        let c = compare(&a.source.source, &b.source.source);
        if c != 0 {
            return ord(c);
        }
        let c = compare(&a.source.original_source, &b.source.original_source);
        if c != 0 {
            return ord(c);
        }
        let c = compare(&a.source.kind, &b.source.kind);
        if c != 0 {
            return ord(c);
        }
        a.style.cmp(&b.style).then(a.index.cmp(&b.index))
    });
    refs
}

fn ord(c: i32) -> std::cmp::Ordering {
    if c < 0 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}

// ---------------------------------------------------------------------------
// printSortedItems (shared.js)
// ---------------------------------------------------------------------------

fn print_sorted_items<'a>(
    sorted_items: &OuterGroups<'a>,
    original_items: &[ImportExportItem],
    source_text: &str,
    all_comments: &[Comment],
    newline: &str,
) -> CompactString {
    // Build sorted string: groups within outer joined by newline, outers by double newline
    let nl = newline;
    let double_nl = {
        let mut s = CompactString::new("");
        s.push_str(nl);
        s.push_str(nl);
        s
    };

    let mut sorted = CompactString::new("");
    let mut first_outer = true;
    for groups in sorted_items.iter() {
        if !first_outer {
            sorted.push_str(double_nl.as_str());
        }
        first_outer = false;
        let mut first_inner = true;
        for group in groups.iter() {
            if !first_inner {
                sorted.push_str(nl);
            }
            first_inner = false;
            let mut first_item = true;
            for item in group.iter() {
                if !first_item {
                    sorted.push_str(nl);
                }
                first_item = false;
                sorted.push_str(item.code.as_str());
            }
        }
    }

    let mut result = sorted;

    // Edge case: if last sorted item needs_newline and there's code on the same
    // line as the last original item, add a newline.
    let flat_sorted: SmallVec<[&ImportExportItem; 8]> = sorted_items
        .iter()
        .flat_map(|groups| groups.iter().flat_map(|g| g.iter().copied()))
        .collect();

    if let (Some(last_sorted), Some(last_original)) = (flat_sorted.last(), original_items.last())
        && last_sorted.needs_newline
    {
        let lo_end = last_original.node_end;
        let lo_end_line = shared::line_of(source_text, lo_end);

        // Find first token after lo that is NOT a line comment and NOT a
        // block comment ending on the same line as lo
        let next = find_next_valid_token(source_text, all_comments, lo_end, lo_end_line);

        if let Some(next_start) = next
            && shared::line_of(source_text, next_start) == lo_end_line
        {
            result.push_str(newline);
        }
    }

    result
}

/// Find first token after `offset` that is:
/// - not a line comment
/// - not a block comment whose `.loc.end.line == base_line`
fn find_next_valid_token(
    source_text: &str,
    all_comments: &[Comment],
    offset: u32,
    base_line: u32,
) -> Option<u32> {
    for c in all_comments {
        if c.span.start < offset {
            continue;
        }
        match c.kind {
            CommentKind::Line => continue, // skip line comments
            CommentKind::SingleLineBlock | CommentKind::MultiLineBlock => {
                let end_line = shared::line_of(source_text, c.span.end);
                if end_line == base_line {
                    continue;
                } // block comment on same line
            }
        }
        return Some(c.span.start);
    }
    // Check for real (non-comment) code
    let after = source_text.get(offset as usize..)?;
    let skip = after.find(|c: char| !c.is_whitespace())?;
    Some(offset + skip as u32)
}

// ---------------------------------------------------------------------------
// maybeReportSorting (shared.js)
// ---------------------------------------------------------------------------

fn maybe_report_sorting(
    source_text: &str,
    line_index: &LineIndex,
    sorted: CompactString,
    start: u32,
    end: u32,
    rule_kind: RuleKind,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    let original = source_text.get(start as usize..end as usize).unwrap_or("");
    if original == sorted.as_str() {
        return;
    }
    diagnostics.push(Diagnostic {
        rule_name: match rule_kind {
            RuleKind::Imports => "imports",
            RuleKind::Exports => "exports",
        },
        message_id: "sort",
        loc: line_index.loc_for_span(source_text, Span::new(start, end)),
        fix: Some(DiagnosticFix {
            start,
            end,
            replacement: sorted,
        }),
    });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn module_export_name(name: &ModuleExportName<'_>) -> CompactString {
    match name {
        ModuleExportName::IdentifierName(id) => CompactString::from(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => CompactString::from(id.name.as_str()),
        ModuleExportName::StringLiteral(lit) => CompactString::from(lit.value.as_str()),
    }
}

pub(crate) fn kind_rank(kind: ImportOrExportKind) -> u8 {
    match kind {
        ImportOrExportKind::Type => 0,
        ImportOrExportKind::Value => 1,
    }
}

fn import_kind_str(kind: ImportOrExportKind) -> &'static str {
    match kind {
        ImportOrExportKind::Type => "type",
        ImportOrExportKind::Value => "value",
    }
}
