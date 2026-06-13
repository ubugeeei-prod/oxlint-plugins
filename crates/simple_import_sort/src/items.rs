//! Builders and sort keys for the chunk `Item` type.

use oxc_ast::ast::{
    ExportAllDeclaration, ExportNamedDeclaration, ImportDeclaration, ImportDeclarationSpecifier,
    ImportOrExportKind,
};
use oxlint_plugins_carton::CompactString;
use regex::Regex;

use crate::scanner::span_text;
use crate::specifiers::sort_import_specifiers_in_code;
use crate::specifiers::sort_export_specifiers_in_code;
use crate::types::{EXPORT_STYLE, Item, SIDE_EFFECT_STYLE, SimpleImportSortOptions};

pub(crate) fn item_from_import(
    source_text: &str,
    declaration: &ImportDeclaration<'_>,
    options: &SimpleImportSortOptions,
    index: usize,
) -> Item {
    let original = declaration.source.value.as_str();
    let source_original = CompactString::from(original);
    let source_key = source_sort_key(original);
    let kind_rank_value = kind_rank(declaration.import_kind);
    let style = import_style(declaration);
    let code = sort_import_specifiers_in_code(
        source_text,
        span_text(source_text, declaration.span),
        declaration,
    );
    let (outer_group, inner_group) = import_group(style, kind_rank_value, original, options);
    Item {
        span: declaration.span,
        code,
        source_original,
        source_key,
        kind_rank: kind_rank_value,
        style,
        index,
        outer_group,
        inner_group,
    }
}

pub(crate) fn item_from_named_export(
    source_text: &str,
    declaration: &ExportNamedDeclaration<'_>,
    index: usize,
) -> Item {
    let original = declaration
        .source
        .as_ref()
        .expect("export-from declaration has source")
        .value
        .as_str();
    Item {
        span: declaration.span,
        code: sort_export_specifiers_in_code(
            source_text,
            span_text(source_text, declaration.span),
            &declaration.specifiers,
        ),
        source_original: CompactString::from(original),
        source_key: source_sort_key(original),
        kind_rank: kind_rank(declaration.export_kind),
        style: EXPORT_STYLE,
        index,
        outer_group: 0,
        inner_group: 0,
    }
}

pub(crate) fn item_from_all_export(
    source_text: &str,
    declaration: &ExportAllDeclaration<'_>,
    index: usize,
) -> Item {
    let original = declaration.source.value.as_str();
    Item {
        span: declaration.span,
        code: CompactString::from(span_text(source_text, declaration.span)),
        source_original: CompactString::from(original),
        source_key: source_sort_key(original),
        kind_rank: kind_rank(declaration.export_kind),
        style: EXPORT_STYLE,
        index,
        outer_group: 0,
        inner_group: 0,
    }
}

pub(crate) fn import_style(declaration: &ImportDeclaration<'_>) -> u8 {
    let Some(specifiers) = &declaration.specifiers else {
        return SIDE_EFFECT_STYLE;
    };
    let Some(first) = specifiers.first() else {
        return 3;
    };
    match first {
        ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => 1,
        ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => 2,
        ImportDeclarationSpecifier::ImportSpecifier(_) => 3,
    }
}

pub(crate) fn import_group(
    style: u8,
    kind_rank: u8,
    original: &str,
    options: &SimpleImportSortOptions,
) -> (usize, usize) {
    if options.import_groups.is_empty() {
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

    let match_source = import_match_source(style, kind_rank, original);
    let mut best: Option<(usize, usize, usize)> = None;
    for (outer_index, group) in options.import_groups.iter().enumerate() {
        for (inner_index, pattern) in group.iter().enumerate() {
            let Ok(regex) = Regex::new(pattern.as_str()) else {
                continue;
            };
            let Some(matched) = regex.find(match_source.as_str()) else {
                continue;
            };
            let len = matched.end().saturating_sub(matched.start());
            if best.is_none_or(|(_, _, best_len)| len > best_len) {
                best = Some((outer_index, inner_index, len));
            }
        }
    }
    best.map(|(outer, inner, _)| (outer, inner))
        .unwrap_or((options.import_groups.len(), 0))
}

fn import_match_source(style: u8, kind_rank: u8, original: &str) -> CompactString {
    let mut source = CompactString::new("");
    if style == SIDE_EFFECT_STYLE {
        source.push('\0');
    }
    source.push_str(original);
    if kind_rank == 0 {
        source.push('\0');
    }
    source
}

fn is_package_source(source: &str) -> bool {
    let Some(first) = source.chars().next() else {
        return false;
    };
    first == '@' || first == '_' || first.is_ascii_alphanumeric()
}

fn source_sort_key(source: &str) -> CompactString {
    let mut out = CompactString::new("");
    let mut normalized = CompactString::from(source);
    if normalized.chars().all(|ch| ch == '.' || ch == '/') && normalized.ends_with('.') {
        normalized.push('/');
    }
    if normalized.chars().all(|ch| ch == '.' || ch == '/') && normalized.ends_with('/') {
        normalized.push(',');
    }
    for ch in normalized.chars() {
        match ch {
            '.' => out.push('_'),
            '/' => out.push('-'),
            '_' => out.push('.'),
            '-' => out.push('/'),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn kind_rank(kind: ImportOrExportKind) -> u8 {
    match kind {
        ImportOrExportKind::Type => 0,
        ImportOrExportKind::Value => 1,
    }
}
