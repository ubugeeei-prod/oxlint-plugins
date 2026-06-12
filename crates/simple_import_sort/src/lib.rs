#![doc = "Rust implementation of eslint-plugin-simple-import-sort rule logic."]

use std::cmp::Ordering;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ExportAllDeclaration, ExportNamedDeclaration, ExportSpecifier, ImportDeclaration,
    ImportDeclarationSpecifier, ImportOrExportKind, ModuleExportName, Statement,
};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

pub const RULE_NAMES: [&str; 2] = ["exports", "imports"];

const SIDE_EFFECT_STYLE: u8 = 0;
const EXPORT_STYLE: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimpleImportSortOptions {
    pub import_groups: SmallVec<[SmallVec<[CompactString; 4]>; 8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuleKind {
    Imports,
    Exports,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Item {
    span: Span,
    code: CompactString,
    source_original: CompactString,
    source_key: CompactString,
    kind_rank: u8,
    style: u8,
    index: usize,
    outer_group: usize,
    inner_group: usize,
}

struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    fn new(source_text: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn loc_for_span(&self, source_text: &str, span: Span) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source_text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        let line_index = line_index.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = source_text[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line_index + 1) as u32, column as u32)
    }
}

pub fn implemented_simple_import_sort_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_simple_import_sort(
    source_text: &str,
    filename: &str,
    options: &SimpleImportSortOptions,
) -> SmallVec<[Diagnostic; 8]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let line_index = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();
    scan_import_chunks(
        source_text,
        &line_index,
        &parser_return.program.body,
        options,
        &mut diagnostics,
    );
    scan_export_chunks(
        source_text,
        &line_index,
        &parser_return.program.body,
        &mut diagnostics,
    );
    diagnostics
}

fn scan_import_chunks(
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

fn scan_export_chunks(
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
    declaration: &ExportNamedDeclaration<'_>,
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

fn item_from_import(
    source_text: &str,
    declaration: &ImportDeclaration<'_>,
    options: &SimpleImportSortOptions,
    index: usize,
) -> Item {
    let original = declaration.source.value.as_str();
    let source_original = CompactString::from(original);
    let source_key = source_sort_key(original);
    let kind_rank = kind_rank(declaration.import_kind);
    let style = import_style(declaration);
    let code = sort_import_specifiers_in_code(
        source_text,
        span_text(source_text, declaration.span),
        declaration,
    );
    let (outer_group, inner_group) = import_group(style, kind_rank, original, options);
    Item {
        span: declaration.span,
        code,
        source_original,
        source_key,
        kind_rank,
        style,
        index,
        outer_group,
        inner_group,
    }
}

fn item_from_named_export(
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

fn item_from_all_export(
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

fn import_style(declaration: &ImportDeclaration<'_>) -> u8 {
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

fn import_group(
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

fn kind_rank(kind: ImportOrExportKind) -> u8 {
    match kind {
        ImportOrExportKind::Type => 0,
        ImportOrExportKind::Value => 1,
    }
}

fn sort_import_specifiers_in_code(
    source_text: &str,
    original: &str,
    declaration: &ImportDeclaration<'_>,
) -> CompactString {
    let Some(specifiers) = &declaration.specifiers else {
        return CompactString::from(original);
    };
    let mut named: SmallVec<[(CompactString, CompactString, u8, CompactString); 8]> =
        SmallVec::new();
    for specifier in specifiers {
        if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier {
            named.push((
                module_name(&specifier.imported),
                CompactString::from(specifier.local.name.as_str()),
                kind_rank(specifier.import_kind),
                CompactString::from(span_text(source_text, specifier.span)),
            ));
        }
    }
    if named.len() <= 1 {
        return CompactString::from(original);
    }
    named.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    replace_braced_specifiers(original, named.iter().map(|item| item.3.as_str()))
}

fn sort_export_specifiers_in_code(
    source_text: &str,
    original: &str,
    specifiers: &[ExportSpecifier<'_>],
) -> CompactString {
    if specifiers.len() <= 1 {
        return CompactString::from(original);
    }
    let mut named: SmallVec<[(CompactString, CompactString, u8, CompactString); 8]> =
        SmallVec::new();
    for specifier in specifiers {
        named.push((
            module_name(&specifier.exported),
            module_name(&specifier.local),
            kind_rank(specifier.export_kind),
            CompactString::from(span_text(source_text, specifier.span)),
        ));
    }
    named.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    replace_braced_specifiers(original, named.iter().map(|item| item.3.as_str()))
}

fn replace_braced_specifiers<'a>(
    original: &str,
    sorted_specifiers: impl Iterator<Item = &'a str>,
) -> CompactString {
    let Some(open) = original.find('{') else {
        return CompactString::from(original);
    };
    let Some(close) = original.rfind('}') else {
        return CompactString::from(original);
    };
    if close <= open {
        return CompactString::from(original);
    }
    let mut out = CompactString::new("");
    out.push_str(&original[..open + 1]);
    out.push(' ');
    for (index, specifier) in sorted_specifiers.enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(specifier.trim());
    }
    out.push(' ');
    out.push_str(&original[close..]);
    out
}

fn module_name(name: &ModuleExportName<'_>) -> CompactString {
    match name {
        ModuleExportName::IdentifierName(identifier) => {
            CompactString::from(identifier.name.as_str())
        }
        ModuleExportName::IdentifierReference(identifier) => {
            CompactString::from(identifier.name.as_str())
        }
        ModuleExportName::StringLiteral(literal) => CompactString::from(literal.value.as_str()),
    }
}

fn guess_newline(source_text: &str) -> &str {
    if source_text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn span_text(source_text: &str, span: Span) -> &str {
    source_text
        .get(span.start as usize..span.end as usize)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{SimpleImportSortOptions, scan_simple_import_sort};

    #[test]
    fn sorts_import_chunks_and_specifiers() {
        let source = [
            "import z from 'z';",
            "import { beta, alpha as renamed } from 'pkg';",
            "import fs from 'node:fs';",
            "import './setup';",
            "import local from './local';",
        ]
        .join("\n");
        let diagnostics =
            scan_simple_import_sort(&source, "fixture.js", &SimpleImportSortOptions::default());

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_name, "imports");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("fix")
                .replacement
                .as_str(),
            [
                "import './setup';",
                "",
                "import fs from 'node:fs';",
                "",
                "import { alpha as renamed, beta } from 'pkg';",
                "import z from 'z';",
                "",
                "import local from './local';",
            ]
            .join("\n")
        );
    }

    #[test]
    fn sorts_export_chunks_and_local_specifiers() {
        let source = [
            "export { zed } from 'z';",
            "export * from 'a';",
            "export { d, a as c, b };",
        ]
        .join("\n");
        let diagnostics =
            scan_simple_import_sort(&source, "fixture.js", &SimpleImportSortOptions::default());

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].rule_name, "exports");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("fix")
                .replacement
                .as_str(),
            ["export * from 'a';", "export { zed } from 'z';"].join("\n")
        );
        assert_eq!(
            diagnostics[1]
                .fix
                .as_ref()
                .expect("fix")
                .replacement
                .as_str(),
            "export { b, a as c, d };"
        );
    }
}
