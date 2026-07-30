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

pub const RULE_NAMES: [&str; 7] = [
    "no-deprecated-astro-canonicalurl",
    "no-deprecated-astro-fetchcontent",
    "no-deprecated-astro-resolve",
    "no-deprecated-getentrybyslug",
    "no-set-html-directive",
    "no-set-text-directive",
    "prefer-class-list-directive",
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    /// UTF-8 byte offset into the original `.astro` source.
    pub start: u32,
    /// UTF-8 byte offset into the original `.astro` source.
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    /// UTF-8 byte offset into the original source.
    pub start: u32,
    /// UTF-8 byte offset into the original source.
    pub end: u32,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

pub fn implemented_astro_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

/// Scans Astro frontmatter and the template body.
///
/// Frontmatter and segmented template expressions are parsed with Oxc. The
/// surrounding Astro markup is handled by a conservative attribute/element
/// segmenter instead of being treated as JavaScript.
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
            body_offset: source_text.len(),
        }
    } else {
        match frontmatter(source_text) {
            Some(frontmatter) => frontmatter,
            None if starts_with_frontmatter_delimiter(source_text) => return SmallVec::new(),
            None => Frontmatter {
                source: "",
                offset: 0,
                body_offset: 0,
            },
        }
    };

    let line_index = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();
    let Some(global_astro_shadowed) = scan_script(
        frontmatter.source,
        frontmatter.offset,
        source_text,
        options,
        &line_index,
        &mut diagnostics,
        false,
    ) else {
        return SmallVec::new();
    };
    if !options.frontmatter_only {
        scan_template(
            source_text,
            frontmatter.body_offset,
            options,
            &line_index,
            &mut diagnostics,
            global_astro_shadowed,
        );
    }
    diagnostics.sort_by(|left, right| {
        left.loc
            .start_line
            .cmp(&right.loc.start_line)
            .then(left.loc.start_column.cmp(&right.loc.start_column))
            .then(left.rule_name.cmp(right.rule_name))
    });
    diagnostics
}

fn scan_script(
    script: &str,
    source_offset: u32,
    source_text: &str,
    options: &AstroOptions,
    line_index: &LineIndex,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
    inherited_astro_shadow: bool,
) -> Option<bool> {
    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, script, SourceType::ts().with_module(true)).parse();
    if !parser_return.errors.is_empty() {
        return None;
    }
    let semantic_return = SemanticBuilder::new().build(&parser_return.program);
    if !semantic_return.errors.is_empty() {
        return None;
    }
    let scoping = semantic_return.semantic.scoping();
    let global_astro_shadowed =
        inherited_astro_shadow || scoping.get_root_binding("Astro".into()).is_some();
    let mut scanner = Scanner {
        source_text,
        source_offset,
        options,
        line_index,
        scoping,
        global_astro_shadowed: inherited_astro_shadow,
        diagnostics,
    };
    scanner.visit_program(&parser_return.program);
    Some(global_astro_shadowed)
}

fn has_astro_extension(filename: &str) -> bool {
    filename
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("astro"))
}

fn starts_with_frontmatter_delimiter(source: &str) -> bool {
    let bom_len = usize::from(source.starts_with('\u{feff}')) * '\u{feff}'.len_utf8();
    read_line(source, bom_len).is_some_and(|line| {
        line.text.trim_end_matches([' ', '\t']) == "---" && line.next > line.start
    })
}

#[derive(Clone, Copy)]
struct Frontmatter<'a> {
    source: &'a str,
    offset: u32,
    body_offset: usize,
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
                body_offset: line.next,
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

struct Scanner<'s, 'd> {
    source_text: &'s str,
    source_offset: u32,
    options: &'s AstroOptions,
    line_index: &'s LineIndex,
    scoping: &'s Scoping,
    global_astro_shadowed: bool,
    diagnostics: &'d mut SmallVec<[Diagnostic; 8]>,
}

impl Scanner<'_, '_> {
    fn is_global_astro(&self, expression: &Expression<'_>) -> bool {
        if self.global_astro_shadowed {
            return false;
        }
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
            start: span.start,
            end: span.end,
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
                    replacement: CompactString::new("glob"),
                });
                self.report("no-deprecated-astro-fetchcontent", span, fix);
            }
            "resolve" => {
                self.report("no-deprecated-astro-resolve", span, None);
            }
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for Scanner<'_, '_> {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AttributeKind {
    Normal,
    Shorthand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AttributeValueKind {
    Expression,
    Template,
    Other,
}

#[derive(Clone, Copy, Debug)]
struct AttributeValue {
    span: Span,
    kind: AttributeValueKind,
}

#[derive(Clone, Debug)]
struct TemplateAttribute<'a> {
    name: &'a str,
    name_span: Span,
    span: Span,
    kind: AttributeKind,
    value: Option<AttributeValue>,
}

#[derive(Clone, Debug)]
struct TemplateElement<'a> {
    name: &'a str,
    opening_span: Span,
    attributes: SmallVec<[TemplateAttribute<'a>; 8]>,
    self_closing: bool,
    closing_span: Option<Span>,
    children_span: Option<Span>,
}

fn scan_template(
    source: &str,
    body_offset: usize,
    options: &AstroOptions,
    line_index: &LineIndex,
    diagnostics: &mut SmallVec<[Diagnostic; 8]>,
    global_astro_shadowed: bool,
) {
    let mut cursor = body_offset;
    let mut elements: SmallVec<[TemplateElement<'_>; 16]> = SmallVec::new();
    let mut stack: SmallVec<[usize; 16]> = SmallVec::new();
    let mut expressions: SmallVec<[Span; 16]> = SmallVec::new();

    while cursor < source.len() {
        if source[cursor..].starts_with("<!--") {
            cursor = source[cursor + 4..]
                .find("-->")
                .map_or(source.len(), |relative| cursor + 4 + relative + 3);
            continue;
        }
        if source.as_bytes()[cursor] == b'{'
            && let Some(end) = balanced_brace_end(source, cursor)
        {
            if end > cursor + 2 {
                expressions.push(span_from_usize(cursor + 1, end - 1));
            }
            cursor = end;
            continue;
        }
        if source.as_bytes()[cursor] == b'<' {
            if source[cursor..].starts_with("</") {
                if let Some((name, closing_span, end)) = parse_closing_tag(source, cursor) {
                    if let Some(position) = stack
                        .iter()
                        .rposition(|index| elements[*index].name == name)
                    {
                        let element_index = stack.remove(position);
                        stack.truncate(position);
                        let opening_end = elements[element_index].opening_span.end;
                        elements[element_index].closing_span = Some(closing_span);
                        elements[element_index].children_span =
                            Some(Span::new(opening_end, closing_span.start));
                    }
                    cursor = end;
                    continue;
                }
            } else if let Some((element, end, attribute_expressions)) =
                parse_opening_tag(source, cursor)
            {
                expressions.extend(attribute_expressions);
                let self_closing = element.self_closing;
                elements.push(element);
                if !self_closing {
                    stack.push(elements.len() - 1);
                }
                cursor = end;
                continue;
            }
        }
        cursor = next_char_boundary(source, cursor);
    }

    for expression_span in expressions {
        let start = expression_span.start as usize;
        let end = expression_span.end as usize;
        if start >= end || end > source.len() {
            continue;
        }
        let _ = scan_script(
            &source[start..end],
            expression_span.start,
            source,
            options,
            line_index,
            diagnostics,
            global_astro_shadowed,
        );
    }

    let mut reporter = TemplateReporter {
        source,
        options,
        line_index,
        diagnostics,
    };
    for element in &elements {
        let set_text_count = element
            .attributes
            .iter()
            .filter(|attribute| attribute.name == "set:text")
            .count();
        for attribute in &element.attributes {
            match attribute.name {
                "set:html" => reporter.report(
                    "no-set-html-directive",
                    "unexpected",
                    attribute.name_span,
                    None,
                ),
                "set:text" => {
                    let fix = (set_text_count == 1)
                        .then(|| set_text_fix(source, element, attribute))
                        .flatten();
                    reporter.report(
                        "no-set-text-directive",
                        "disallow",
                        attribute.name_span,
                        fix,
                    );
                }
                "class"
                    if matches!(
                        attribute.value,
                        Some(AttributeValue {
                            kind: AttributeValueKind::Expression | AttributeValueKind::Template,
                            ..
                        })
                    ) || attribute.kind == AttributeKind::Shorthand =>
                {
                    let (start, end, replacement) = if attribute.kind == AttributeKind::Shorthand {
                        (
                            attribute.span.start,
                            attribute.span.start,
                            CompactString::new("class:list="),
                        )
                    } else {
                        (
                            attribute.name_span.end,
                            attribute.name_span.end,
                            CompactString::new(":list"),
                        )
                    };
                    reporter.report(
                        "prefer-class-list-directive",
                        "unexpected",
                        attribute.name_span,
                        Some(DiagnosticFix {
                            start,
                            end,
                            replacement,
                        }),
                    );
                }
                _ => {}
            }
        }
    }
}

struct TemplateReporter<'a, 'd> {
    source: &'a str,
    options: &'a AstroOptions,
    line_index: &'a LineIndex,
    diagnostics: &'d mut SmallVec<[Diagnostic; 8]>,
}

impl TemplateReporter<'_, '_> {
    fn report(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        fix: Option<DiagnosticFix>,
    ) {
        if !self.options.has_rule(rule_name) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            start: span.start,
            end: span.end,
            loc: self.line_index.loc_for_span(self.source, span),
            fix,
        });
    }
}

fn set_text_fix(
    source: &str,
    element: &TemplateElement<'_>,
    attribute: &TemplateAttribute<'_>,
) -> Option<DiagnosticFix> {
    let value = attribute.value?;
    let value_text = &source[value.span.start as usize..value.span.end as usize];
    let mut rendered_value = CompactString::new("");
    if value.kind == AttributeValueKind::Template {
        rendered_value.push('{');
        rendered_value.push_str(value_text);
        rendered_value.push('}');
    } else {
        rendered_value.push_str(value_text);
    }

    let opening_start = element.opening_span.start as usize;
    let opening_end = element.opening_span.end as usize;
    let attribute_start = attribute.span.start as usize;
    let attribute_end = attribute.span.end as usize;
    let mut replacement = CompactString::new("");
    replacement.push_str(&source[opening_start..attribute_start]);
    replacement.push_str(&source[attribute_end..opening_end]);

    let replacement_end = if element.self_closing {
        if !replacement.ends_with("/>") {
            return None;
        }
        replacement.remove(replacement.len() - 2);
        replacement.push_str(&rendered_value);
        replacement.push_str("</");
        replacement.push_str(element.name);
        replacement.push('>');
        element.opening_span.end
    } else {
        let children = element.children_span?;
        if !source[children.start as usize..children.end as usize]
            .trim()
            .is_empty()
        {
            return None;
        }
        let closing = element.closing_span?;
        replacement.push_str(&rendered_value);
        replacement.push_str(&source[closing.start as usize..closing.end as usize]);
        closing.end
    };

    Some(DiagnosticFix {
        start: element.opening_span.start,
        end: replacement_end,
        replacement,
    })
}

fn parse_opening_tag<'a>(
    source: &'a str,
    start: usize,
) -> Option<(TemplateElement<'a>, usize, SmallVec<[Span; 4]>)> {
    if !source[start..].starts_with('<')
        || source[start..].starts_with("</")
        || source[start..].starts_with("<!")
        || source[start..].starts_with("<?")
        || source[start..].starts_with("<>")
    {
        return None;
    }
    let mut cursor = start + 1;
    let name_start = cursor;
    while cursor < source.len() && is_tag_name_byte(source.as_bytes()[cursor]) {
        cursor += 1;
    }
    if cursor == name_start {
        return None;
    }
    let name = &source[name_start..cursor];
    let mut attributes = SmallVec::new();
    let mut expressions = SmallVec::new();

    loop {
        cursor = skip_ascii_whitespace(source, cursor);
        if cursor >= source.len() {
            return None;
        }
        if source[cursor..].starts_with("/>") {
            let end = cursor + 2;
            return Some((
                TemplateElement {
                    name,
                    opening_span: span_from_usize(start, end),
                    attributes,
                    self_closing: true,
                    closing_span: None,
                    children_span: None,
                },
                end,
                expressions,
            ));
        }
        if source.as_bytes()[cursor] == b'>' {
            let end = cursor + 1;
            return Some((
                TemplateElement {
                    name,
                    opening_span: span_from_usize(start, end),
                    attributes,
                    self_closing: false,
                    closing_span: None,
                    children_span: None,
                },
                end,
                expressions,
            ));
        }

        let attribute_start = cursor;
        if source.as_bytes()[cursor] == b'{' {
            let end = balanced_brace_end(source, cursor)?;
            let inner = source[cursor + 1..end - 1].trim();
            if is_identifier_name(inner) {
                let relative = source[cursor + 1..end - 1].find(inner)?;
                let name_start = cursor + 1 + relative;
                attributes.push(TemplateAttribute {
                    name: inner,
                    name_span: span_from_usize(name_start, name_start + inner.len()),
                    span: span_from_usize(attribute_start, end),
                    kind: AttributeKind::Shorthand,
                    value: Some(AttributeValue {
                        span: span_from_usize(cursor + 1, end - 1),
                        kind: AttributeValueKind::Expression,
                    }),
                });
                expressions.push(span_from_usize(cursor + 1, end - 1));
            }
            cursor = end;
            continue;
        }

        let name_start = cursor;
        while cursor < source.len() && is_attribute_name_byte(source.as_bytes()[cursor]) {
            cursor += 1;
        }
        if cursor == name_start {
            cursor = next_char_boundary(source, cursor);
            continue;
        }
        let attribute_name = &source[name_start..cursor];
        let name_span = span_from_usize(name_start, cursor);
        cursor = skip_ascii_whitespace(source, cursor);
        let mut value = None;
        if cursor < source.len() && source.as_bytes()[cursor] == b'=' {
            cursor = skip_ascii_whitespace(source, cursor + 1);
            let value_start = cursor;
            if cursor >= source.len() {
                return None;
            }
            match source.as_bytes()[cursor] {
                b'{' => {
                    cursor = balanced_brace_end(source, cursor)?;
                    value = Some(AttributeValue {
                        span: span_from_usize(value_start, cursor),
                        kind: AttributeValueKind::Expression,
                    });
                    expressions.push(span_from_usize(value_start + 1, cursor - 1));
                }
                b'`' => {
                    cursor = quoted_end(source, cursor, b'`')?;
                    value = Some(AttributeValue {
                        span: span_from_usize(value_start, cursor),
                        kind: AttributeValueKind::Template,
                    });
                    collect_template_interpolations(source, value_start, cursor, &mut expressions);
                }
                b'\'' | b'"' => {
                    let quote = source.as_bytes()[cursor];
                    cursor = quoted_end(source, cursor, quote)?;
                    value = Some(AttributeValue {
                        span: span_from_usize(value_start, cursor),
                        kind: AttributeValueKind::Other,
                    });
                }
                _ => {
                    while cursor < source.len()
                        && !source.as_bytes()[cursor].is_ascii_whitespace()
                        && !matches!(source.as_bytes()[cursor], b'>' | b'/')
                    {
                        cursor += 1;
                    }
                    value = Some(AttributeValue {
                        span: span_from_usize(value_start, cursor),
                        kind: AttributeValueKind::Other,
                    });
                }
            }
        }
        attributes.push(TemplateAttribute {
            name: attribute_name,
            name_span,
            span: span_from_usize(attribute_start, cursor),
            kind: AttributeKind::Normal,
            value,
        });
    }
}

fn parse_closing_tag(source: &str, start: usize) -> Option<(&str, Span, usize)> {
    let mut cursor = start + 2;
    let name_start = cursor;
    while cursor < source.len() && is_tag_name_byte(source.as_bytes()[cursor]) {
        cursor += 1;
    }
    if cursor == name_start {
        return None;
    }
    let name = &source[name_start..cursor];
    cursor = skip_ascii_whitespace(source, cursor);
    if cursor >= source.len() || source.as_bytes()[cursor] != b'>' {
        return None;
    }
    let end = cursor + 1;
    Some((name, span_from_usize(start, end), end))
}

fn collect_template_interpolations(
    source: &str,
    start: usize,
    end: usize,
    expressions: &mut SmallVec<[Span; 4]>,
) {
    let mut cursor = start + 1;
    while cursor + 1 < end {
        if source[cursor..].starts_with("${")
            && let Some(expression_end) = balanced_brace_end(source, cursor + 1)
        {
            expressions.push(span_from_usize(cursor + 2, expression_end - 1));
            cursor = expression_end;
            continue;
        }
        cursor = next_char_boundary(source, cursor);
    }
}

fn balanced_brace_end(source: &str, start: usize) -> Option<usize> {
    if source.as_bytes().get(start) != Some(&b'{') {
        return None;
    }
    let mut cursor = start + 1;
    let mut depth = 1_u32;
    while cursor < source.len() {
        if source[cursor..].starts_with("//") {
            cursor = source[cursor + 2..]
                .find(['\n', '\r', '\u{2028}', '\u{2029}'])
                .map_or(source.len(), |relative| cursor + 2 + relative);
            continue;
        }
        if source[cursor..].starts_with("/*") {
            cursor = source[cursor + 2..]
                .find("*/")
                .map_or(source.len(), |relative| cursor + 2 + relative + 2);
            continue;
        }
        match source.as_bytes()[cursor] {
            b'\'' | b'"' | b'`' => {
                cursor = quoted_end(source, cursor, source.as_bytes()[cursor])?;
            }
            b'{' => {
                depth += 1;
                cursor += 1;
            }
            b'}' => {
                depth -= 1;
                cursor += 1;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => cursor = next_char_boundary(source, cursor),
        }
    }
    None
}

fn quoted_end(source: &str, start: usize, quote: u8) -> Option<usize> {
    let mut cursor = start + 1;
    while cursor < source.len() {
        match source.as_bytes()[cursor] {
            b'\\' => {
                cursor += 1;
                if cursor < source.len() {
                    cursor = next_char_boundary(source, cursor);
                }
            }
            byte if byte == quote => return Some(cursor + 1),
            _ => cursor = next_char_boundary(source, cursor),
        }
    }
    None
}

fn skip_ascii_whitespace(source: &str, mut cursor: usize) -> usize {
    while cursor < source.len() && source.as_bytes()[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    cursor
}

fn next_char_boundary(source: &str, cursor: usize) -> usize {
    cursor + source[cursor..].chars().next().map_or(1, char::len_utf8)
}

fn is_tag_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'$' | b'.' | b'-')
}

fn is_attribute_name_byte(byte: u8) -> bool {
    !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'>' | b'/' | b'{' | b'}')
}

fn is_identifier_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphanumeric())
}

fn span_from_usize(start: usize, end: usize) -> Span {
    Span::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
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
