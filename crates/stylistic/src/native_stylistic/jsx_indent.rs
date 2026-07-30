//! Native implementation of stable `@stylistic/jsx-indent` v5.10.0.
//!
//! The deprecated upstream rule is intentionally context-sensitive: JSX
//! opening tags inherit indentation from the token or structural expression
//! immediately before them, closing tags align with their opening peer,
//! expression containers and text are one level deeper, and optional
//! attribute checks inspect the last expression token. Oxc supplies the AST
//! parent graph while the shared Stylistic lexer preserves exact source ranges.

use std::{collections::BTreeMap, fmt::Write as _};

use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::JSXAttributeValue};
use oxc_parser::Parser;
use oxc_semantic::{AstNodes, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::node::NodeId;
use serde_json::Value;

use super::lexer::{Token, tokenize};
use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-indent";
const MESSAGE_ID: &str = "wrongIndent";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IndentKind {
    Space,
    Tab,
}

impl IndentKind {
    const fn character(self) -> u8 {
        match self {
            Self::Space => b' ',
            Self::Tab => b'\t',
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Space => "space",
            Self::Tab => "tab",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Options {
    kind: IndentKind,
    size: i32,
    check_attributes: bool,
    indent_logical_expressions: bool,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let values = match options {
            Value::Array(values) => values.as_slice(),
            Value::Null => &[],
            value => std::slice::from_ref(value),
        };
        let (kind, size) = match values.first() {
            Some(Value::String(value)) if value == "tab" => (IndentKind::Tab, 1),
            Some(Value::Number(value)) => (
                IndentKind::Space,
                value
                    .as_i64()
                    .and_then(|value| i32::try_from(value).ok())
                    .unwrap_or(4),
            ),
            _ => (IndentKind::Space, 4),
        };
        let object = values.get(1).and_then(Value::as_object);
        Self {
            kind,
            size,
            check_attributes: object
                .and_then(|value| value.get("checkAttributes"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            indent_logical_expressions: object
                .and_then(|value| value.get("indentLogicalExpressions"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }

    fn indent(self, count: i32) -> String {
        let count = usize::try_from(count.max(0)).unwrap_or(0);
        std::iter::repeat_n(char::from(self.kind.character()), count).collect()
    }
}

pub(crate) fn check_jsx_indent(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_json(options);
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        if parse_and_check(source, source_type, options, diagnostics) {
            return;
        }
        check_do_expression_fallback(source, options, diagnostics);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, options, diagnostics) {
            return;
        }
    }
    check_do_expression_fallback(source, options, diagnostics);
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    options: Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }
    let semantic_return = SemanticBuilder::new().build(&parsed.program);
    if !semantic_return.errors.is_empty() {
        return false;
    }

    let semantic = semantic_return.semantic;
    let nodes = semantic.nodes();
    let tokens = tokenize(source)
        .into_iter()
        .filter(|token| !token.kind.is_comment())
        .collect::<Vec<_>>();
    let mut checker = Checker {
        source,
        lines: LineMap::new(source),
        nodes,
        tokens,
        options,
        diagnostics,
    };
    checker.run();
    true
}

struct Checker<'ast, 'source, 'diagnostics> {
    source: &'source str,
    lines: LineMap,
    nodes: &'ast AstNodes<'ast>,
    tokens: Vec<Token>,
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl Checker<'_, '_, '_> {
    fn run(&mut self) {
        for node in self.nodes.iter() {
            let span = node.span();
            match node.kind() {
                AstKind::JSXOpeningElement(_) | AstKind::JSXOpeningFragment(_) => {
                    self.check_opening(node.id(), span);
                }
                AstKind::JSXClosingElement(_) | AstKind::JSXClosingFragment(_) => {
                    self.check_closing(node.id(), span);
                }
                AstKind::JSXExpressionContainer(_) => {
                    self.check_expression_container(node.id(), span);
                }
                AstKind::JSXText(_) => self.check_text(node.id(), span),
                AstKind::JSXAttribute(attribute) if self.options.check_attributes => {
                    self.check_attribute(attribute);
                }
                AstKind::ReturnStatement(statement) => {
                    self.check_return(node.id(), statement.span, statement.argument.as_ref());
                }
                _ => {}
            }
        }
    }

    fn check_opening(&mut self, node_id: NodeId, span: Span) {
        let Some(start) = usize::try_from(span.start).ok() else {
            return;
        };
        let anchor = self.opening_anchor(node_id, start);
        let anchor_indent = self.indent_at(anchor, false);
        let anchor_line = self.lines.line(anchor);
        let node_line = self.lines.line(start);
        let right_logical = self.is_right_in_logical_expression(node_id);
        let alternate = self.is_alternate_in_conditional_expression(node_id);
        let added = if anchor_line == node_line || right_logical || alternate {
            0
        } else {
            self.options.size
        };
        let expected = anchor_indent + added;
        let gotten = self.indent_at(start, false);
        if gotten == expected || !self.is_first_in_line(start) {
            return;
        }
        if right_logical
            && !self.options.indent_logical_expressions
            && gotten - expected == self.options.size
        {
            return;
        }
        self.report(span, expected, gotten, self.indent_fix(start, expected));
    }

    fn check_closing(&mut self, node_id: NodeId, span: Span) {
        let parent = self.nodes.parent_kind(node_id);
        let opening_start = match parent {
            AstKind::JSXElement(element) => element.opening_element.span.start,
            AstKind::JSXFragment(fragment) => fragment.opening_fragment.span.start,
            _ => return,
        };
        let expected = self.indent_at_u32(opening_start, false);
        self.check_simple(span, expected, false);
    }

    fn check_expression_container(&mut self, node_id: NodeId, span: Span) {
        let parent = self.nodes.parent_node(node_id);
        let expected = self.indent_at_u32(parent.span().start, false) + self.options.size;
        self.check_simple(span, expected, false);
    }

    fn check_text(&mut self, node_id: NodeId, span: Span) {
        if !matches!(
            self.nodes.parent_kind(node_id),
            AstKind::JSXElement(_) | AstKind::JSXFragment(_)
        ) {
            return;
        }
        let expected = self.indent_at_u32(self.nodes.parent_node(node_id).span().start, false)
            + self.options.size;
        let Some(start) = usize::try_from(span.start).ok() else {
            return;
        };
        let Some(end) = usize::try_from(span.end).ok() else {
            return;
        };
        let Some(text) = self.source.get(start..end) else {
            return;
        };
        let mut mismatches = Vec::new();
        for content_start in line_content_starts(text) {
            let tail = &text[content_start..];
            let line_end = tail
                .char_indices()
                .find_map(|(index, character)| is_line_terminator(character).then_some(index))
                .unwrap_or(tail.len());
            let line = &tail[..line_end];
            let Some(non_whitespace) = line
                .char_indices()
                .find_map(|(index, character)| (!matches!(character, ' ' | '\t')).then_some(index))
            else {
                continue;
            };
            let gotten = line.as_bytes()[..non_whitespace]
                .iter()
                .take_while(|&&byte| byte == self.options.kind.character())
                .count();
            let gotten = i32::try_from(gotten).unwrap_or(i32::MAX);
            if gotten != expected {
                mismatches.push(gotten);
            }
        }
        if mismatches.is_empty() {
            return;
        }

        let replacement = rewrite_text_indentation(text, expected, self.options);
        for gotten in mismatches {
            self.report(
                span,
                expected,
                gotten,
                Some(LintFix::replace_range(
                    TextRange::new(span.start, span.end),
                    replacement.clone(),
                )),
            );
        }
    }

    fn check_attribute(&mut self, attribute: &oxc_ast::ast::JSXAttribute<'_>) {
        let Some(JSXAttributeValue::ExpressionContainer(container)) = attribute.value.as_ref()
        else {
            return;
        };
        let Some(last_index) = self.tokens.iter().rposition(|token| {
            token.start >= container.span.start as usize && token.end <= container.span.end as usize
        }) else {
            return;
        };
        let Some(previous_index) = last_index.checked_sub(1) else {
            return;
        };
        let last = self.tokens[last_index];
        let previous = self.tokens[previous_index];
        if self.lines.line(previous.start) != self.lines.line(last.start) {
            return;
        }
        let expected = if self.lines.line(attribute.name.span().start as usize)
            == self.lines.line(previous.start)
        {
            0
        } else {
            self.indent_at(attribute.name.span().start as usize, false)
        };
        self.check_simple(
            Span::new(previous.start as u32, previous.end as u32),
            expected,
            false,
        );
    }

    fn check_return(
        &mut self,
        node_id: NodeId,
        span: Span,
        argument: Option<&oxc_ast::ast::Expression<'_>>,
    ) {
        let Some(argument) = argument else {
            return;
        };
        let Some(argument_text) = self.source_slice(argument.span()) else {
            return;
        };
        if !argument_text
            .trim_start()
            .trim_start_matches('(')
            .trim_start()
            .starts_with('<')
            || !self
                .nodes
                .ancestor_kinds(node_id)
                .any(|kind| matches!(kind, AstKind::Function(_)))
        {
            return;
        }
        let expected = self.indent_at_u32(span.start, false);
        let Some(end) = usize::try_from(span.end).ok() else {
            return;
        };
        let last_line_start = self.lines.line_start(end.saturating_sub(1));
        let gotten = self.indent_at(last_line_start, false);
        if gotten == expected {
            return;
        }
        let Some(last_line) = self.source.get(last_line_start..end) else {
            return;
        };
        let content = last_line.trim_start_matches([' ', '\t']);
        let mut replacement = String::with_capacity(content.len() + expected.max(0) as usize + 1);
        replacement.push('\n');
        replacement.push_str(&self.options.indent(expected));
        replacement.push_str(content);
        self.report(
            span,
            expected,
            gotten,
            Some(LintFix::replace_range(
                byte_range(self.line_break_start(last_line_start), end),
                replacement,
            )),
        );
    }

    fn check_simple(&mut self, span: Span, expected: i32, exclude_commas: bool) {
        let Some(start) = usize::try_from(span.start).ok() else {
            return;
        };
        let gotten = self.indent_at(start, exclude_commas);
        if gotten == expected || !self.is_first_in_line(start) {
            return;
        }
        self.report(span, expected, gotten, self.indent_fix(start, expected));
    }

    fn report(&mut self, span: Span, needed: i32, gotten: i32, fix: Option<LintFix>) {
        let characters = if needed == 1 {
            "character"
        } else {
            "characters"
        };
        let message = wrong_indent_message(self.options.kind, needed, gotten, characters);
        let data = BTreeMap::from([
            ("needed".to_owned(), decimal_string(needed)),
            ("type".to_owned(), self.options.kind.name().to_owned()),
            ("characters".to_owned(), characters.to_owned()),
            ("gotten".to_owned(), decimal_string(gotten)),
        ]);
        let suggestions = fix.map_or_else(Vec::new, |fix| {
            std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message: message.clone(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect()
        });
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message,
            data,
            range: TextRange::new(span.start, span.end),
            suggestions,
        });
    }

    fn opening_anchor(&self, node_id: NodeId, start: usize) -> usize {
        if let Some(text_node) = self.nodes.iter().find(|node| {
            matches!(node.kind(), AstKind::JSXText(_))
                && usize::try_from(node.span().end).ok() == Some(start)
        }) {
            return usize::try_from(self.nodes.parent_node(text_node.id()).span().start)
                .unwrap_or(0);
        }

        let Some(previous) = self.previous_token(start) else {
            return start;
        };
        let previous_text = previous.text(self.source);
        if previous_text == ":" {
            if let Some(conditional) = self
                .nodes
                .ancestor_kinds(node_id)
                .find(|kind| matches!(kind, AstKind::ConditionalExpression(_)))
            {
                return conditional.span().start as usize;
            }
        }
        if previous_text == "," {
            if let Some(mut anchor_id) = self.deepest_node_at(previous.start) {
                if matches!(
                    self.nodes.kind(anchor_id),
                    AstKind::StringLiteral(_) | AstKind::JSXText(_)
                ) {
                    anchor_id = self.nodes.parent_id(anchor_id);
                }
                return self.expression_container_expression_start(anchor_id);
            }
        }
        previous.start
    }

    fn expression_container_expression_start(&self, node_id: NodeId) -> usize {
        match self.nodes.kind(node_id) {
            AstKind::JSXExpressionContainer(container) => {
                usize::try_from(container.expression.span().start).unwrap_or(0)
            }
            kind => usize::try_from(kind.span().start).unwrap_or(0),
        }
    }

    fn deepest_node_at(&self, offset: usize) -> Option<NodeId> {
        let offset = u32::try_from(offset).ok()?;
        self.nodes
            .iter()
            .filter(|node| {
                let span = node.span();
                span.start <= offset && offset < span.end
            })
            .min_by_key(|node| node.span().size())
            .map(|node| node.id())
    }

    fn is_right_in_logical_expression(&self, node_id: NodeId) -> bool {
        if self.options.indent_logical_expressions {
            return false;
        }
        let jsx_id = self.nodes.parent_id(node_id);
        let mut expression_id = self.nodes.parent_id(jsx_id);
        while matches!(
            self.nodes.kind(expression_id),
            AstKind::ParenthesizedExpression(_)
        ) {
            expression_id = self.nodes.parent_id(expression_id);
        }
        matches!(
            self.nodes.kind(expression_id),
            AstKind::LogicalExpression(expression)
                if contains_span(expression.right.span(), self.nodes.kind(jsx_id).span())
        )
    }

    fn is_alternate_in_conditional_expression(&self, node_id: NodeId) -> bool {
        let jsx_id = self.nodes.parent_id(node_id);
        let mut expression_id = self.nodes.parent_id(jsx_id);
        while matches!(
            self.nodes.kind(expression_id),
            AstKind::ParenthesizedExpression(_)
        ) {
            expression_id = self.nodes.parent_id(expression_id);
        }
        let alternate = matches!(
            self.nodes.kind(expression_id),
            AstKind::ConditionalExpression(expression)
                if contains_span(expression.alternate.span(), self.nodes.kind(jsx_id).span())
                    && self.first_jsx_start(expression.consequent.span()).is_some_and(|start|
                        self.lines.line(start) == self.lines.line(expression.span.start as usize)
                    )
        );
        alternate
            && self
                .previous_token(self.nodes.kind(node_id).span().start as usize)
                .is_none_or(|token| token.text(self.source) != "(")
    }

    fn previous_token(&self, start: usize) -> Option<&Token> {
        self.tokens.iter().rev().find(|token| token.end <= start)
    }

    fn first_jsx_start(&self, within: Span) -> Option<usize> {
        self.nodes
            .iter()
            .filter_map(|node| {
                matches!(
                    node.kind(),
                    AstKind::JSXElement(_) | AstKind::JSXFragment(_)
                )
                .then_some(node.span())
            })
            .filter(|span| contains_span(within, *span))
            .map(|span| span.start as usize)
            .min()
    }

    fn is_first_in_line(&self, start: usize) -> bool {
        self.previous_token(start)
            .is_some_and(|token| self.lines.line(token.start) != self.lines.line(start))
    }

    fn indent_fix(&self, start: usize, expected: i32) -> Option<LintFix> {
        Some(LintFix::replace_range(
            byte_range(self.lines.line_start(start), start),
            self.options.indent(expected),
        ))
    }

    fn indent_at_u32(&self, offset: u32, exclude_commas: bool) -> i32 {
        usize::try_from(offset).map_or(0, |offset| self.indent_at(offset, exclude_commas))
    }

    fn indent_at(&self, offset: usize, exclude_commas: bool) -> i32 {
        let start = self.lines.line_start(offset);
        let count = self.source.as_bytes()[start..]
            .iter()
            .take_while(|&&byte| {
                byte == self.options.kind.character() || (exclude_commas && byte == b',')
            })
            .count();
        i32::try_from(count).unwrap_or(i32::MAX)
    }

    fn source_slice(&self, span: Span) -> Option<&str> {
        self.source
            .get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
    }

    fn line_break_start(&self, content_start: usize) -> usize {
        let prefix = &self.source.as_bytes()[..content_start];
        if prefix.ends_with(b"\r\n") {
            content_start.saturating_sub(2)
        } else if prefix.ends_with(b"\n") || prefix.ends_with(b"\r") {
            content_start.saturating_sub(1)
        } else if prefix.ends_with("\u{2028}".as_bytes()) || prefix.ends_with("\u{2029}".as_bytes())
        {
            content_start.saturating_sub(3)
        } else {
            content_start
        }
    }
}

fn check_do_expression_fallback(
    source: &str,
    options: Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if !source.contains("do {") {
        return;
    }
    let lines = LineMap::new(source);
    let mut block_indents = Vec::<i32>::new();
    for line in 0..lines.len() {
        let start = lines.starts[line];
        let end = lines.line_end(line, source.len());
        let raw = &source[start..end];
        let content = raw.trim_end_matches(is_line_terminator);
        let trimmed = content.trim_start_matches([' ', '\t']);
        let gotten = indentation(raw, options.kind);

        if trimmed.starts_with('}') {
            block_indents.pop();
        }
        if let Some(&block_indent) = block_indents.last()
            && let Some(relative) = trimmed.find('<')
            && !trimmed[relative..].starts_with("</")
            && let Some(close) = trimmed[relative..].find('>')
        {
            let expected = block_indent + options.size;
            if gotten != expected {
                let tag_start = start + content.len() - trimmed.len() + relative;
                let tag_end = tag_start + close + 1;
                let characters = if expected == 1 {
                    "character"
                } else {
                    "characters"
                };
                let message = wrong_indent_message(options.kind, expected, gotten, characters);
                diagnostics.push(LintDiagnostic {
                    rule_name: RULE.to_owned(),
                    message_id: MESSAGE_ID.to_owned(),
                    message: message.clone(),
                    data: BTreeMap::from([
                        ("needed".to_owned(), decimal_string(expected)),
                        ("type".to_owned(), options.kind.name().to_owned()),
                        ("characters".to_owned(), characters.to_owned()),
                        ("gotten".to_owned(), decimal_string(gotten)),
                    ]),
                    range: byte_range(tag_start, tag_end),
                    suggestions: std::iter::once(LintSuggestion {
                        message_id: MESSAGE_ID.to_owned(),
                        message,
                        fixes: std::iter::once(LintFix::replace_range(
                            byte_range(start, tag_start),
                            options.indent(expected),
                        ))
                        .collect(),
                    })
                    .collect(),
                });
            }
        }
        if trimmed.contains("do {")
            || trimmed.starts_with("if ") && trimmed.ends_with('{')
            || trimmed.starts_with("if(") && trimmed.ends_with('{')
            || trimmed.starts_with("else") && trimmed.ends_with('{')
            || trimmed.starts_with("} else") && trimmed.ends_with('{')
        {
            block_indents.push(gotten);
        }
    }
}

fn indentation(line: &str, kind: IndentKind) -> i32 {
    let count = line
        .as_bytes()
        .iter()
        .take_while(|&&byte| byte == kind.character())
        .count();
    i32::try_from(count).unwrap_or(i32::MAX)
}

fn rewrite_text_indentation(text: &str, expected: i32, options: Options) -> String {
    let indent = options.indent(expected);
    let mut output = String::with_capacity(text.len() + indent.len());
    let mut cursor = 0;
    for content_start in line_content_starts(text) {
        let absolute = content_start;
        let tail = &text[absolute..];
        let whitespace = tail
            .char_indices()
            .take_while(|(_, character)| matches!(character, ' ' | '\t'))
            .map(|(index, character)| index + character.len_utf8())
            .last()
            .unwrap_or(0);
        let Some(first) = tail[whitespace..].chars().next() else {
            continue;
        };
        if is_line_terminator(first) {
            continue;
        }
        output.push_str(&text[cursor..absolute]);
        output.push_str(&indent);
        cursor = absolute + whitespace;
    }
    output.push_str(&text[cursor..]);
    output
}

fn line_content_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut characters = text.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if character == '\r' {
            if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                characters.next();
                starts.push(index + 2);
            } else {
                starts.push(index + 1);
            }
        } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
            starts.push(index + character.len_utf8());
        }
    }
    starts
}

fn wrong_indent_message(kind: IndentKind, needed: i32, gotten: i32, characters: &str) -> String {
    let mut message = String::with_capacity(72);
    message.push_str("Expected indentation of ");
    let _ = write!(message, "{needed}");
    message.push(' ');
    message.push_str(kind.name());
    message.push(' ');
    message.push_str(characters);
    message.push_str(" but found ");
    let _ = write!(message, "{gotten}");
    message.push('.');
    message
}

fn decimal_string(value: i32) -> String {
    let mut output = String::with_capacity(11);
    let _ = write!(output, "{value}");
    output
}

const fn is_line_terminator(character: char) -> bool {
    matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}')
}

struct LineMap {
    starts: Vec<usize>,
}

impl LineMap {
    fn new(source: &str) -> Self {
        let mut starts = Vec::with_capacity(source.lines().count().saturating_add(1));
        starts.push(0);
        let mut characters = source.char_indices().peekable();
        while let Some((index, character)) = characters.next() {
            if character == '\r' {
                if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                    characters.next();
                    starts.push(index + 2);
                } else {
                    starts.push(index + 1);
                }
            } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
                starts.push(index + character.len_utf8());
            }
        }
        Self { starts }
    }

    fn line(&self, offset: usize) -> usize {
        self.starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1)
    }

    fn line_start(&self, offset: usize) -> usize {
        self.starts[self.line(offset)]
    }

    fn len(&self) -> usize {
        self.starts.len()
    }

    fn line_end(&self, line: usize, source_len: usize) -> usize {
        self.starts.get(line + 1).copied().unwrap_or(source_len)
    }
}

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

const fn contains_span(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-indent-v5.10.0.json"
    ));

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Generated {
        commit: String,
        source_sha256: String,
        rule_source_sha256: String,
        parser_matrix_source_sha256: String,
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        fixable_invalid: usize,
        unfixable_invalid: usize,
        total: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TestCase {
        code: String,
        #[serde(default)]
        options: Value,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        recursive_output: Option<String>,
        #[serde(default)]
        diagnostics: Vec<ExpectedDiagnostic>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        data: ExpectedData,
        range: [u32; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    struct ExpectedData {
        needed: i32,
        #[serde(rename = "type")]
        kind: String,
        characters: String,
        gotten: i32,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedFix {
        range: [u32; 2],
        replacement_text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(FIXTURE).expect("valid jsx-indent fixture")
    }

    fn run(source: &str, options: &Value, filename: &str) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_indent(source, Some(filename), options, &mut diagnostics);
        diagnostics
    }

    fn first_fix(diagnostic: &LintDiagnostic) -> Option<&LintFix> {
        diagnostic
            .suggestions
            .first()
            .and_then(|suggestion| suggestion.fixes.first())
    }

    fn apply_one_pass(source: &str, diagnostics: &[LintDiagnostic]) -> String {
        let mut fixes = diagnostics
            .iter()
            .enumerate()
            .filter_map(|(index, diagnostic)| first_fix(diagnostic).map(|fix| (index, fix)))
            .collect::<Vec<_>>();
        fixes.sort_by_key(|(index, fix)| (fix.range.start, fix.range.end, *index));
        let mut accepted = Vec::new();
        let mut last_end = None;
        for (_, fix) in fixes {
            if last_end.is_some_and(|end| fix.range.start <= end) {
                continue;
            }
            last_end = Some(fix.range.end);
            accepted.push(fix);
        }
        let mut output = source.to_owned();
        for fix in accepted.into_iter().rev() {
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        output
    }

    fn recursive_fix(source: &str, options: &Value) -> String {
        let mut output = source.to_owned();
        for _ in 0..10 {
            let diagnostics = run(&output, options, "fixture.tsx");
            let next = apply_one_pass(&output, &diagnostics);
            if next == output {
                return output;
            }
            output = next;
        }
        output
    }

    fn single_option(option: Value) -> Value {
        Value::Array(std::iter::once(option).collect())
    }

    #[test]
    fn pins_complete_upstream_inventory_and_source_hashes() {
        let fixture = fixture();
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(
            fixture.generated.source_sha256,
            "0469c5c8ae40e881bfb21abecaf5c08813955fefc99608526fc460b3fc16fbf2"
        );
        assert_eq!(
            fixture.generated.rule_source_sha256,
            "845ae761d471cfb80dde5745e18b41554c6e5f810d74abd129e88ba18af0885f"
        );
        assert_eq!(
            fixture.generated.parser_matrix_source_sha256,
            "64dd12d67eac1eadf8a5a93de02bbb76c1d764c0ec7ebbdaae0c45389b52435c"
        );
        let inventory = fixture.generated.inventory;
        assert_eq!(
            (
                inventory.valid,
                inventory.invalid,
                inventory.diagnostics,
                inventory.fixable_invalid,
                inventory.unfixable_invalid,
                inventory.total,
            ),
            (106, 65, 84, 65, 0, 171)
        );
    }

    #[test]
    fn accepts_every_authored_valid_case() {
        let fixture = fixture();
        for (index, test_case) in fixture.valid.iter().enumerate() {
            let diagnostics = run(&test_case.code, &test_case.options, "fixture.tsx");
            assert!(
                diagnostics.is_empty(),
                "valid case {index}\n{}\n{diagnostics:#?}",
                test_case.code,
            );
        }
    }

    #[test]
    fn replays_every_invalid_diagnostic_range_and_fix_exactly() {
        let fixture = fixture();
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, &test_case.options, "fixture.tsx");
            assert_eq!(
                diagnostics.len(),
                test_case.diagnostics.len(),
                "invalid case {index}\n{}",
                test_case.code
            );
            for (diagnostic_index, (actual, expected)) in
                diagnostics.iter().zip(&test_case.diagnostics).enumerate()
            {
                let expected_needed = decimal_string(expected.data.needed);
                let expected_gotten = decimal_string(expected.data.gotten);
                assert_eq!(
                    actual.message_id, expected.message_id,
                    "case {index} diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.message, expected.message,
                    "case {index} diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.range,
                    TextRange::new(expected.range[0], expected.range[1]),
                    "case {index} diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.data.get("needed"),
                    Some(&expected_needed),
                    "case {index} diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.data.get("type"),
                    Some(&expected.data.kind),
                    "case {index} diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.data.get("characters"),
                    Some(&expected.data.characters),
                    "case {index} diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.data.get("gotten"),
                    Some(&expected_gotten),
                    "case {index} diagnostic {diagnostic_index}"
                );
                match (&expected.fix, first_fix(actual)) {
                    (Some(expected), Some(actual)) => {
                        assert_eq!(
                            actual.range,
                            TextRange::new(expected.range[0], expected.range[1]),
                            "case {index} diagnostic {diagnostic_index}"
                        );
                        assert_eq!(
                            actual.replacement_text, expected.replacement_text,
                            "case {index} diagnostic {diagnostic_index}"
                        );
                    }
                    (None, None) => {}
                    _ => panic!("case {index} diagnostic {diagnostic_index} fix presence diverged"),
                }
            }
            assert_eq!(
                apply_one_pass(&test_case.code, &diagnostics),
                test_case.output.as_deref().unwrap_or(&test_case.code),
                "invalid case {index} first pass"
            );
            assert_eq!(
                recursive_fix(&test_case.code, &test_case.options),
                test_case
                    .recursive_output
                    .as_deref()
                    .unwrap_or(&test_case.code),
                "invalid case {index} recursive output"
            );
        }
    }

    #[test]
    fn covers_unicode_tsx_comments_options_and_all_line_terminators() {
        let source = "const 日本語: string = '😀';\nconst view = (\n  <App>\n  <Child value={日本語} />\n  </App>\n);";
        let diagnostics = run(source, &single_option(Value::from(2)), "fixture.tsx");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].message,
            "Expected indentation of 4 space characters but found 2."
        );
        assert_eq!(
            first_fix(&diagnostics[0]).map(|fix| fix.replacement_text.as_str()),
            Some("    ")
        );

        for terminator in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let mut source = String::from("<App>");
            source.push_str(terminator);
            source.push_str("<Child />");
            source.push_str(terminator);
            source.push_str("</App>");
            assert!(
                run(&source, &single_option(Value::from(0)), "fixture.jsx").is_empty(),
                "{terminator:?}"
            );
            source.replace_range(source.len() - "</App>".len().., "  </App>");
            assert_eq!(
                run(&source, &single_option(Value::from(0)), "fixture.jsx").len(),
                1,
                "{terminator:?}"
            );
        }

        let source = "<App>\n\t{/* comment */}\n\t<Child />\n</App>";
        assert!(run(source, &single_option(Value::from("tab")), "fixture.jsx").is_empty());
        assert!(run("<Broken>", &Value::Null, "fixture.jsx").is_empty());
        assert!(run("<App />", &Value::Null, "fixture.ts").is_empty());
    }
}
