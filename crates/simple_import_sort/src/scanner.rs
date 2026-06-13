//! Top-level scan loop that groups imports/exports into chunks and reports
//! sort diagnostics with rewrites.

use std::cmp::Ordering;

use oxc_ast::ast::Statement;
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::items::{item_from_all_export, item_from_import, item_from_named_export};
use crate::specifiers::sort_export_specifiers_in_code;
use crate::types::{
    Diagnostic, DiagnosticFix, Item, LineIndex, RuleKind, SIDE_EFFECT_STYLE, SimpleImportSortOptions,
};

pub(crate) fn scan_import_chunks(
    source_text: &str,
    line_index: &LineIndex,
    statements: &[Statement<'_>],
    options: &SimpleImportSortOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    let mut chunk: SmallVec<[Item; 16]> = SmallVec::new();
    for statement in statements {
        if let Statement::ImportDeclaration(declaration) = statement {
            let index = chunk.len();
            chunk.push(item_from_import(source_text, declaration, options, index));
        } else {
            report_chunk(
                source_text,
                line_index,
                &chunk,
                RuleKind::Imports,
                diagnostics,
            );
            chunk.clear();
        }
    }
    report_chunk(
        source_text,
        line_index,
        &chunk,
        RuleKind::Imports,
        diagnostics,
    );
}

pub(crate) fn scan_export_chunks(
    source_text: &str,
    line_index: &LineIndex,
    statements: &[Statement<'_>],
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    let mut chunk: SmallVec<[Item; 16]> = SmallVec::new();
    for statement in statements {
        match statement {
            Statement::ExportNamedDeclaration(declaration)
                if declaration.source.is_some() && declaration.declaration.is_none() =>
            {
                let index = chunk.len();
                chunk.push(item_from_named_export(source_text, declaration, index));
            }
            Statement::ExportAllDeclaration(declaration) => {
                let index = chunk.len();
                chunk.push(item_from_all_export(source_text, declaration, index));
            }
            Statement::ExportNamedDeclaration(declaration)
                if declaration.source.is_none()
                    && declaration.declaration.is_none()
                    && declaration.specifiers.len() > 1 =>
            {
                report_chunk(
                    source_text,
                    line_index,
                    &chunk,
                    RuleKind::Exports,
                    diagnostics,
                );
                chunk.clear();
                report_named_export_specifiers(source_text, line_index, declaration, diagnostics);
            }
            _ => {
                report_chunk(
                    source_text,
                    line_index,
                    &chunk,
                    RuleKind::Exports,
                    diagnostics,
                );
                chunk.clear();
            }
        }
    }
    report_chunk(
        source_text,
        line_index,
        &chunk,
        RuleKind::Exports,
        diagnostics,
    );
}

fn report_chunk(
    source_text: &str,
    line_index: &LineIndex,
    chunk: &[Item],
    rule_kind: RuleKind,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    if chunk.is_empty() {
        return;
    }
    let sorted = print_sorted_chunk(chunk, guess_newline(source_text));
    let start = chunk.first().expect("chunk has first item").span.start;
    let end = chunk.last().expect("chunk has last item").span.end;
    let original = source_text
        .get(start as usize..end as usize)
        .unwrap_or_default();
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

fn report_named_export_specifiers(
    source_text: &str,
    line_index: &LineIndex,
    declaration: &oxc_ast::ast::ExportNamedDeclaration<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
) {
    let original = span_text(source_text, declaration.span);
    let sorted = sort_export_specifiers_in_code(source_text, original, &declaration.specifiers);
    if sorted.as_str() == original {
        return;
    }
    diagnostics.push(Diagnostic {
        rule_name: "exports",
        message_id: "sort",
        loc: line_index.loc_for_span(source_text, declaration.span),
        fix: Some(DiagnosticFix {
            start: declaration.span.start,
            end: declaration.span.end,
            replacement: sorted,
        }),
    });
}

fn print_sorted_chunk(items: &[Item], newline: &str) -> CompactString {
    let mut sorted: SmallVec<[Item; 16]> = items.iter().cloned().collect();
    sorted.sort_by(compare_items);

    let mut out = CompactString::new("");
    let mut previous_outer_group = None;
    for item in sorted {
        if !out.is_empty() {
            out.push_str(newline);
            if previous_outer_group.is_some_and(|group| group != item.outer_group) {
                out.push_str(newline);
            }
        }
        out.push_str(item.code.as_str());
        previous_outer_group = Some(item.outer_group);
    }
    out
}

fn compare_items(a: &Item, b: &Item) -> Ordering {
    a.outer_group
        .cmp(&b.outer_group)
        .then(a.inner_group.cmp(&b.inner_group))
        .then_with(|| {
            if a.style == SIDE_EFFECT_STYLE && b.style == SIDE_EFFECT_STYLE {
                a.index.cmp(&b.index)
            } else if a.style == SIDE_EFFECT_STYLE {
                Ordering::Less
            } else if b.style == SIDE_EFFECT_STYLE {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .then(a.source_key.cmp(&b.source_key))
        .then(a.source_original.cmp(&b.source_original))
        .then(a.kind_rank.cmp(&b.kind_rank))
        .then(a.style.cmp(&b.style))
        .then(a.index.cmp(&b.index))
}

fn guess_newline(source_text: &str) -> &str {
    if source_text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

pub(crate) fn span_text(source_text: &str, span: Span) -> &str {
    source_text
        .get(span.start as usize..span.end as usize)
        .unwrap_or_default()
}
