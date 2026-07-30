#![doc = "Rust implementation of selected eslint-plugin-astro rule logic."]

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ComputedMemberExpression, Expression, ImportDeclaration, ImportDeclarationSpecifier,
    ModuleExportName, StaticMemberExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::Scoping;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub const RULE_NAMES: [&str; 3] = [
    "no-deprecated-astro-canonicalurl",
    "no-deprecated-astro-fetchcontent",
    "no-deprecated-getentrybyslug",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AstroOptions {
    /// Empty means all implemented rules are enabled.
    pub rule_names: SmallVec<[CompactString; 4]>,
    /// The caller already extracted the TypeScript frontmatter segment.
    pub frontmatter_only: bool,
}

impl AstroOptions {
    fn has_rule(&self, name: &str) -> bool {
        self.rule_names.is_empty() || self.rule_names.iter().any(|rule| rule == name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    /// UTF-8 byte offset into the original `.astro` source.
    pub start: u32,
    /// UTF-8 byte offset into the original `.astro` source.
    pub end: u32,
    pub replacement: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

pub fn implemented_astro_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

/// Scans the first Astro frontmatter block.
///
/// Direct callers can pass raw `.astro` source; Oxlint's JavaScript-plugin path
/// passes its extracted frontmatter through [`AstroOptions::frontmatter_only`].
/// This core deliberately parses only that TypeScript segment with Oxc.
/// Template rules remain out of scope until a separate expression segmenter is
/// added.
pub fn scan_astro(
    source_text: &str,
    filename: &str,
    options: &AstroOptions,
) -> SmallVec<[Diagnostic; 8]> {
    if !has_astro_extension(filename) {
        return SmallVec::new();
    }
    let frontmatter = if options.frontmatter_only {
        Frontmatter {
            source: source_text,
            offset: 0,
        }
    } else {
        let Some(frontmatter) = frontmatter(source_text) else {
            return SmallVec::new();
        };
        frontmatter
    };

    let allocator = Allocator::default();
    let parser_return = Parser::new(
        &allocator,
        frontmatter.source,
        SourceType::ts().with_module(true),
    )
    .parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }
    let semantic_return = SemanticBuilder::new().build(&parser_return.program);
    if !semantic_return.errors.is_empty() {
        return SmallVec::new();
    }

    let line_index = LineIndex::new(source_text);
    let mut scanner = Scanner {
        source_text,
        source_offset: frontmatter.offset,
        options,
        line_index: &line_index,
        scoping: semantic_return.semantic.scoping(),
        diagnostics: SmallVec::new(),
    };
    scanner.visit_program(&parser_return.program);
    scanner.diagnostics.sort_by(|left, right| {
        left.loc
            .start_line
            .cmp(&right.loc.start_line)
            .then(left.loc.start_column.cmp(&right.loc.start_column))
            .then(left.rule_name.cmp(right.rule_name))
    });
    scanner.diagnostics
}

fn has_astro_extension(filename: &str) -> bool {
    filename
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("astro"))
}

#[derive(Clone, Copy)]
struct Frontmatter<'a> {
    source: &'a str,
    offset: u32,
}

fn frontmatter(source: &str) -> Option<Frontmatter<'_>> {
    let bom_len = usize::from(source.starts_with('\u{feff}')) * '\u{feff}'.len_utf8();
    let opening = read_line(source, bom_len)?;
    if opening.text.trim_end_matches([' ', '\t']) != "---" || opening.next == source.len() {
        return None;
    }

    let content_start = opening.next;
    let mut cursor = content_start;
    while cursor <= source.len() {
        let line = read_line(source, cursor)?;
        if line.text.trim_end_matches([' ', '\t']) == "---" {
            return Some(Frontmatter {
                source: &source[content_start..line.start],
                offset: u32::try_from(content_start).ok()?,
            });
        }
        if line.next == cursor || line.next == source.len() {
            return None;
        }
        cursor = line.next;
    }
    None
}

#[derive(Clone, Copy)]
struct Line<'a> {
    start: usize,
    text: &'a str,
    next: usize,
}

fn read_line(source: &str, start: usize) -> Option<Line<'_>> {
    if start > source.len() || !source.is_char_boundary(start) {
        return None;
    }
    let tail = &source[start..];
    for (relative, ch) in tail.char_indices() {
        let separator_len = match ch {
            '\n' | '\u{2028}' | '\u{2029}' => ch.len_utf8(),
            '\r' => {
                if tail[relative + 1..].starts_with('\n') {
                    2
                } else {
                    1
                }
            }
            _ => continue,
        };
        return Some(Line {
            start,
            text: &source[start..start + relative],
            next: start + relative + separator_len,
        });
    }
    Some(Line {
        start,
        text: tail,
        next: source.len(),
    })
}

struct Scanner<'s> {
    source_text: &'s str,
    source_offset: u32,
    options: &'s AstroOptions,
    line_index: &'s LineIndex,
    scoping: &'s Scoping,
    diagnostics: SmallVec<[Diagnostic; 8]>,
}

impl Scanner<'_> {
    fn is_global_astro(&self, expression: &Expression<'_>) -> bool {
        let Expression::Identifier(identifier) = expression.get_inner_expression() else {
            return false;
        };
        if identifier.name != "Astro" {
            return false;
        }
        let Some(reference_id) = identifier.reference_id.get() else {
            return false;
        };
        let reference = self.scoping.get_reference(reference_id);
        reference.symbol_id().is_none() && reference.is_read()
    }

    fn report(&mut self, rule_name: &'static str, span: Span, fix: Option<DiagnosticFix>) {
        if !self.options.has_rule(rule_name) {
            return;
        }
        let span = shifted_span(span, self.source_offset);
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id: "deprecated",
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix,
        });
    }

    fn check_member(
        &mut self,
        object: &Expression<'_>,
        property: &str,
        span: Span,
        property_span: Option<Span>,
    ) {
        if !self.is_global_astro(object) {
            return;
        }
        match property {
            "canonicalURL" => {
                self.report("no-deprecated-astro-canonicalurl", span, None);
            }
            "fetchContent" => {
                let fix = property_span.map(|span| DiagnosticFix {
                    start: span.start.saturating_add(self.source_offset),
                    end: span.end.saturating_add(self.source_offset),
                    replacement: "glob",
                });
                self.report("no-deprecated-astro-fetchcontent", span, fix);
            }
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for Scanner<'_> {
    fn visit_static_member_expression(&mut self, member: &StaticMemberExpression<'a>) {
        self.check_member(
            &member.object,
            member.property.name.as_str(),
            member.span,
            Some(member.property.span),
        );
        walk::walk_static_member_expression(self, member);
    }

    fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(property) = member.expression.get_inner_expression() {
            self.check_member(&member.object, property.value.as_str(), member.span, None);
        }
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'a>) {
        if declaration.source.value == "astro:content"
            && self.options.has_rule("no-deprecated-getentrybyslug")
            && let Some(specifiers) = &declaration.specifiers
        {
            for specifier in specifiers {
                let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                    continue;
                };
                if matches!(
                    &specifier.imported,
                    ModuleExportName::IdentifierName(identifier)
                        if identifier.name == "getEntryBySlug"
                ) {
                    self.report("no-deprecated-getentrybyslug", specifier.span(), None);
                }
            }
        }
        walk::walk_import_declaration(self, declaration);
    }
}

fn shifted_span(span: Span, offset: u32) -> Span {
    Span::new(
        span.start.saturating_add(offset),
        span.end.saturating_add(offset),
    )
}

struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        let mut cursor = 0;
        while cursor < source.len() {
            let Some(line) = read_line(source, cursor) else {
                break;
            };
            if line.next <= cursor || line.next == source.len() {
                break;
            }
            line_starts.push(line.next);
            cursor = line.next;
        }
        Self { line_starts }
    }

    fn loc_for_span(&self, source: &str, span: Span) -> DiagnosticLoc {
        let (start_line, start_column) = self.position(source, span.start);
        let (end_line, end_column) = self.position(source, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position(&self, source: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source.len());
        let line = self.line_starts.partition_point(|start| *start <= offset);
        let line = line.saturating_sub(1);
        let start = self.line_starts[line];
        let column = source[start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line + 1) as u32, column as u32)
    }
}

#[cfg(test)]
mod tests;
