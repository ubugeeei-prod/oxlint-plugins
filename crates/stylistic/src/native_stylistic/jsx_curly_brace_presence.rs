//! Native implementation of stable `@stylistic/jsx-curly-brace-presence`.
//!
//! The rule checks direct JSX attribute values and children with Oxc's JSX
//! AST. It deliberately preserves upstream's conservative escape, entity,
//! comment, whitespace-adjacency, and `propElementValues` behavior, including
//! the published rule's circular fix for `propElementValues: "never"`.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    Comment,
    ast::{
        JSXAttributeItem, JSXAttributeValue, JSXChild, JSXElement, JSXExpression,
        JSXExpressionContainer, JSXFragment, JSXOpeningElement, JSXText, StringLiteral,
        TemplateLiteral,
    },
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-curly-brace-presence";
const UNNECESSARY_CURLY: (&str, &str) = ("unnecessaryCurly", "Curly braces are unnecessary here.");
const MISSING_CURLY: (&str, &str) = (
    "missingCurly",
    "Need to wrap this literal in a JSX expression.",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Presence {
    Always,
    Never,
    Ignore,
}

impl Presence {
    fn from_value(value: Option<&Value>, fallback: Self) -> Self {
        match value.and_then(Value::as_str) {
            Some("always") => Self::Always,
            Some("never") => Self::Never,
            Some("ignore") => Self::Ignore,
            _ => fallback,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Options {
    props: Presence,
    children: Presence,
    prop_element_values: Presence,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let first = match options {
            Value::Array(values) => values.first(),
            Value::Null => None,
            value => Some(value),
        };
        if let Some(mode) = first.and_then(Value::as_str) {
            let presence =
                Presence::from_value(Some(&Value::String(mode.to_owned())), Presence::Never);
            return Self {
                props: presence,
                children: presence,
                prop_element_values: Presence::Ignore,
            };
        }
        let object = first.and_then(Value::as_object);
        Self {
            props: Presence::from_value(
                object.and_then(|value| value.get("props")),
                Presence::Never,
            ),
            children: Presence::from_value(
                object.and_then(|value| value.get("children")),
                Presence::Never,
            ),
            prop_element_values: Presence::from_value(
                object.and_then(|value| value.get("propElementValues")),
                Presence::Ignore,
            ),
        }
    }
}

pub(crate) fn check_jsx_curly_brace_presence(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_json(options);
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, options, diagnostics);
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

    let start = diagnostics.len();
    let mut visitor = JsxCurlyBracePresence {
        source,
        comments: &parsed.program.comments,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    visitor.diagnostics[start..]
        .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
    true
}

struct JsxCurlyBracePresence<'source, 'comments, 'diagnostics> {
    source: &'source str,
    comments: &'comments [Comment],
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxCurlyBracePresence<'_, '_, '_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'ast>) {
        self.check_opening(&element.opening_element);
        self.check_children(&element.children);
        walk::walk_jsx_element(self, element);
    }

    fn visit_jsx_fragment(&mut self, fragment: &JSXFragment<'ast>) {
        self.check_children(&fragment.children);
        walk::walk_jsx_fragment(self, fragment);
    }
}

impl JsxCurlyBracePresence<'_, '_, '_> {
    fn check_opening(&mut self, opening: &JSXOpeningElement<'_>) {
        for item in &opening.attributes {
            let JSXAttributeItem::Attribute(attribute) = item else {
                continue;
            };
            let Some(value) = &attribute.value else {
                continue;
            };
            match value {
                JSXAttributeValue::StringLiteral(literal)
                    if self.options.props == Presence::Always =>
                {
                    self.check_missing_string_attribute(literal);
                }
                JSXAttributeValue::ExpressionContainer(container) => {
                    if self.options.prop_element_values == Presence::Never
                        && matches!(container.expression, JSXExpression::JSXElement(_))
                    {
                        self.report_unnecessary_jsx(container);
                    } else if self.options.props == Presence::Never
                        && self.attribute_expression_may_be_unnecessary(container)
                    {
                        self.lint_unnecessary(container, ParentKind::Attribute);
                    }
                }
                JSXAttributeValue::Element(element)
                    if self.options.prop_element_values != Presence::Ignore =>
                {
                    self.report_missing_jsx(element.span);
                }
                _ => {}
            }
        }
    }

    fn check_children(&mut self, children: &[JSXChild<'_>]) {
        for (index, child) in children.iter().enumerate() {
            match child {
                JSXChild::Text(text) if self.options.children == Presence::Always => {
                    if self.should_check_missing_text(text, children) {
                        self.report_missing_text(text);
                    }
                }
                JSXChild::ExpressionContainer(container)
                    if self.options.children == Presence::Never
                        && self.should_check_unnecessary_child(container, children) =>
                {
                    self.lint_unnecessary(container, ParentKind::Child);
                }
                _ => {}
            }

            // Keep the explicit index in this source-order loop: the upstream
            // adjacency rules are defined over the exact direct-child list.
            let _ = index;
        }
    }

    fn check_missing_string_attribute(&mut self, literal: &StringLiteral<'_>) {
        let raw = self.text(literal.span);
        if contains_only_html_entities(raw) || is_line_break(raw) {
            return;
        }
        if contains_line_terminator(raw) {
            self.report(literal.span, MISSING_CURLY, None);
            return;
        }
        let inner = strip_matching_quotes(raw).unwrap_or(raw);
        let escaped = escape_double_quotes(&escape_backslashes(inner));
        self.report(literal.span, MISSING_CURLY, Some(braced_quoted(&escaped)));
    }

    fn attribute_expression_may_be_unnecessary(
        &self,
        container: &JSXExpressionContainer<'_>,
    ) -> bool {
        matches!(
            container.expression,
            JSXExpression::StringLiteral(_) | JSXExpression::TemplateLiteral(_)
        )
    }

    fn should_check_unnecessary_child(
        &self,
        container: &JSXExpressionContainer<'_>,
        children: &[JSXChild<'_>],
    ) -> bool {
        let filtered = children
            .iter()
            .filter(|child| !is_whitespace_child(child))
            .collect::<Vec<_>>();
        let adjacent = adjacent_children(container.span, &filtered);
        if adjacent
            .iter()
            .any(|child| matches!(child, JSXChild::ExpressionContainer(_)))
        {
            return false;
        }

        if is_whitespace_expression(container)
            && adjacent.iter().any(|child| {
                matches!(
                    child,
                    JSXChild::ExpressionContainer(_) | JSXChild::Element(_)
                )
            })
        {
            return false;
        }

        if children.len() == 1 && is_whitespace_expression(container) {
            return false;
        }
        true
    }

    fn should_check_missing_text(&self, text: &JSXText<'_>, children: &[JSXChild<'_>]) -> bool {
        let raw = self.text(text.span);
        if is_line_break(raw) || contains_only_html_entities(raw) {
            return false;
        }
        !(children.len() == 1
            && children.first().is_some_and(
                |child| matches!(child, JSXChild::ExpressionContainer(container) if is_whitespace_expression(container)),
            ))
    }

    fn lint_unnecessary(&mut self, container: &JSXExpressionContainer<'_>, parent: ParentKind) {
        if self.comments.iter().any(|comment| {
            comment.span.start >= container.span.start && comment.span.end <= container.span.end
        }) {
            return;
        }

        match &container.expression {
            JSXExpression::StringLiteral(literal) => {
                let value = literal.value.as_str();
                let raw = self.text(literal.span);
                let permitted_whitespace = match parent {
                    ParentKind::Attribute => !is_all_js_whitespace(value),
                    ParentKind::Child => !has_leading_or_trailing_js_whitespace(value),
                };
                if permitted_whitespace
                    && !value.contains("/*")
                    && !needs_escape(raw, parent)
                    && (parent == ParentKind::Child || !contains_quote(value))
                {
                    let replacement = match parent {
                        ParentKind::Attribute => {
                            let inner = strip_matching_quotes(raw).unwrap_or(raw);
                            quoted(inner)
                        }
                        ParentKind::Child => value.to_owned(),
                    };
                    self.report(container.span, UNNECESSARY_CURLY, Some(replacement));
                }
            }
            JSXExpression::TemplateLiteral(template) => {
                if let Some(replacement) = unnecessary_template_replacement(template, parent) {
                    self.report(container.span, UNNECESSARY_CURLY, Some(replacement));
                }
            }
            JSXExpression::JSXElement(element) => {
                self.report(
                    container.span,
                    UNNECESSARY_CURLY,
                    Some(self.text(element.span).to_owned()),
                );
            }
            JSXExpression::JSXFragment(fragment) => {
                self.report(
                    container.span,
                    UNNECESSARY_CURLY,
                    Some(self.text(fragment.span).to_owned()),
                );
            }
            _ => {}
        }
    }

    fn report_unnecessary_jsx(&mut self, container: &JSXExpressionContainer<'_>) {
        let replacement = match &container.expression {
            JSXExpression::JSXElement(element) => self.text(element.span).to_owned(),
            _ => return,
        };
        self.report(container.span, UNNECESSARY_CURLY, Some(replacement));
    }

    fn report_missing_text(&mut self, text: &JSXText<'_>) {
        let raw = self.text(text.span);
        let replacement = if contains_line_terminator(raw) {
            wrap_multiline_text(raw)
        } else {
            braced(&json_string(raw))
        };
        self.report(text.span, MISSING_CURLY, Some(replacement));
    }

    fn report_missing_jsx(&mut self, span: Span) {
        self.report(span, MISSING_CURLY, Some(braced(self.text(span))));
    }

    fn report(&mut self, span: Span, contract: (&str, &str), replacement: Option<String>) {
        let range = text_range(span);
        let suggestions = replacement
            .map(|replacement| LintSuggestion {
                message_id: contract.0.to_owned(),
                message: contract.1.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(range, replacement)).collect(),
            })
            .into_iter()
            .collect();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: contract.0.to_owned(),
            message: contract.1.to_owned(),
            data: BTreeMap::new(),
            range,
            suggestions,
        });
    }

    fn text(&self, span: Span) -> &str {
        self.source
            .get(
                usize::try_from(span.start).unwrap_or(usize::MAX)
                    ..usize::try_from(span.end).unwrap_or(usize::MAX),
            )
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParentKind {
    Attribute,
    Child,
}

fn unnecessary_template_replacement(
    template: &TemplateLiteral<'_>,
    parent: ParentKind,
) -> Option<String> {
    if !template.expressions.is_empty() || template.quasis.len() != 1 {
        return None;
    }
    let quasi = &template.quasis[0];
    let raw = quasi.value.raw.as_str();
    let cooked = quasi.value.cooked.as_ref()?.as_str();
    if raw.contains('\n')
        || has_leading_or_trailing_js_whitespace(raw)
        || needs_escape(raw, parent)
        || contains_quote(cooked)
    {
        return None;
    }
    Some(match parent {
        ParentKind::Attribute => quoted(raw),
        ParentKind::Child => cooked.to_owned(),
    })
}

fn adjacent_children<'a>(span: Span, children: &[&'a JSXChild<'a>]) -> Vec<&'a JSXChild<'a>> {
    let Some(index) = children.iter().position(|child| child.span() == span) else {
        return Vec::new();
    };
    let mut adjacent = Vec::with_capacity(2);
    if index > 0 {
        adjacent.push(children[index - 1]);
    }
    if index + 1 < children.len() {
        adjacent.push(children[index + 1]);
    }
    adjacent
}

fn is_whitespace_child(child: &JSXChild<'_>) -> bool {
    match child {
        JSXChild::ExpressionContainer(container) => is_whitespace_expression(container),
        _ => false,
    }
}

fn is_whitespace_expression(container: &JSXExpressionContainer<'_>) -> bool {
    matches!(
        &container.expression,
        JSXExpression::StringLiteral(literal)
            if !literal.value.is_empty() && is_all_js_whitespace(literal.value.as_str())
    )
}

fn needs_escape(raw: &str, parent: ParentKind) -> bool {
    raw.contains('\\')
        || contains_html_entity(raw)
        || (parent == ParentKind::Child
            && raw
                .chars()
                .any(|character| matches!(character, '{' | '}' | '<' | '>')))
}

fn contains_quote(value: &str) -> bool {
    value.contains(['\'', '"'])
}

fn has_leading_or_trailing_js_whitespace(value: &str) -> bool {
    value.chars().next().is_some_and(is_ecmascript_whitespace)
        || value
            .chars()
            .next_back()
            .is_some_and(is_ecmascript_whitespace)
}

fn contains_line_terminator(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn is_line_break(value: &str) -> bool {
    contains_line_terminator(value) && is_all_js_whitespace(value)
}

fn is_all_js_whitespace(value: &str) -> bool {
    value.chars().all(is_ecmascript_whitespace)
}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{000b}' | '\u{000c}' | '\r' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn contains_html_entity(value: &str) -> bool {
    !html_entity_ranges(value).is_empty()
}

fn contains_only_html_entities(value: &str) -> bool {
    let ranges = html_entity_ranges(value);
    if ranges.is_empty() {
        return false;
    }
    let mut remainder = String::with_capacity(value.len());
    let mut cursor = 0;
    for (start, end) in ranges {
        remainder.push_str(&value[cursor..start]);
        cursor = end;
    }
    remainder.push_str(&value[cursor..]);
    is_all_js_whitespace(&remainder)
}

fn html_entity_ranges(value: &str) -> Vec<(usize, usize)> {
    let bytes = value.as_bytes();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'&' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        let body_start = index;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'#')
        {
            index += 1;
        }
        if index > body_start && bytes.get(index) == Some(&b';') {
            ranges.push((start, index + 1));
            index += 1;
        } else {
            index = start + 1;
        }
    }
    ranges
}

fn wrap_multiline_text(raw: &str) -> String {
    raw.split('\n')
        .map(|line| {
            if is_all_js_whitespace(line) {
                return line.to_owned();
            }
            let first = line
                .char_indices()
                .find_map(|(index, character)| {
                    (!is_ecmascript_whitespace(character)).then_some(index)
                })
                .unwrap_or(line.len());
            let (left_whitespace, text) = line.split_at(first);
            let wrapped = if contains_html_entity(line) {
                wrap_non_html_entities(text)
            } else {
                braced(&json_string(text))
            };
            let mut output = String::with_capacity(left_whitespace.len() + wrapped.len());
            output.push_str(left_whitespace);
            output.push_str(&wrapped);
            output
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap_non_html_entities(text: &str) -> String {
    let ranges = html_entity_ranges(text);
    let mut output = String::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if cursor < start {
            output.push('{');
            output.push_str(&json_string(&text[cursor..start]));
            output.push('}');
        }
        output.push_str(&text[start..end]);
        cursor = end;
    }
    if cursor < text.len() {
        output.push('{');
        output.push_str(&json_string(&text[cursor..]));
        output.push('}');
    }
    output
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

fn quoted(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    output.push_str(value);
    output.push('"');
    output
}

fn braced(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('{');
    output.push_str(value);
    output.push('}');
    output
}

fn braced_quoted(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    output.push_str("{\"");
    output.push_str(value);
    output.push_str("\"}");
    output
}

fn strip_matching_quotes(value: &str) -> Option<&str> {
    let first = value.as_bytes().first().copied()?;
    let last = value.as_bytes().last().copied()?;
    if first == last && matches!(first, b'\'' | b'"') {
        value.get(1..value.len().checked_sub(1)?)
    } else {
        None
    }
}

fn escape_backslashes(value: &str) -> String {
    value.replace('\\', "\\\\")
}

fn escape_double_quotes(value: &str) -> String {
    value.replace("\\\"", "\"").replace('"', "\\\"")
}

fn text_range(span: Span) -> TextRange {
    TextRange::new(span.start, span.end)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the focused JSX option matrices concise"
)]
mod tests {
    use std::collections::BTreeSet;

    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-curly-brace-presence-v5.10.0.json"
    ));

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
    }

    #[derive(Deserialize)]
    struct Generated {
        commit: String,
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        authored_valid: usize,
        authored_invalid: usize,
        exact_diagnostics: usize,
        parser_expanded_total: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TestCase {
        code: String,
        #[serde(default)]
        options: Value,
        parsers: Vec<String>,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
        first_pass_output: Option<String>,
        recursive_output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        data: BTreeMap<String, String>,
        range: [u32; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Debug, Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn run(source: &str, options: &Value, filename: &str) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_curly_brace_presence(source, Some(filename), options, &mut diagnostics);
        diagnostics
    }

    fn apply_compatible_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .filter_map(|suggestion| suggestion.fixes.first())
            .collect::<Vec<_>>();
        fixes.sort_by_key(|fix| (fix.range.start, fix.range.end));
        let mut accepted = Vec::new();
        let mut last_end = None;
        for fix in fixes {
            if last_end.is_some_and(|end| end >= fix.range.start) {
                continue;
            }
            last_end = Some(fix.range.end);
            accepted.push(fix);
        }
        if accepted.is_empty() {
            return None;
        }
        let mut output = source.to_owned();
        for fix in accepted.into_iter().rev() {
            output.replace_range(
                usize::try_from(fix.range.start).expect("fix start")
                    ..usize::try_from(fix.range.end).expect("fix end"),
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    fn recursively_fixed(source: &str, options: &Value, filename: &str) -> Option<String> {
        let mut output = source.to_owned();
        let mut seen = BTreeSet::from([output.clone()]);
        let mut fixed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, options, filename);
            let Some(next) = apply_compatible_fixes(&output, &diagnostics) else {
                return fixed.then_some(output);
            };
            fixed = true;
            if !seen.insert(next.clone()) {
                return Some(next);
            }
            output = next;
        }
        Some(output)
    }

    fn filename(parser: &str) -> &str {
        if parser == "tsx" {
            "fixture.tsx"
        } else {
            "fixture.jsx"
        }
    }

    #[test]
    fn replays_every_authored_case_with_exact_contract_and_fixes() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.generated.inventory.authored_valid, 89);
        assert_eq!(fixture.generated.inventory.authored_invalid, 46);
        assert_eq!(fixture.generated.inventory.exact_diagnostics, 64);
        assert_eq!(fixture.generated.inventory.parser_expanded_total, 263);

        for (index, test_case) in fixture.valid.iter().enumerate() {
            for parser in &test_case.parsers {
                let diagnostics = run(&test_case.code, &test_case.options, filename(parser));
                assert!(
                    diagnostics.is_empty(),
                    "valid case {index} ({parser}): {}\n{diagnostics:#?}",
                    test_case.code
                );
            }
        }

        for (index, test_case) in fixture.invalid.iter().enumerate() {
            for parser in &test_case.parsers {
                let diagnostics = run(&test_case.code, &test_case.options, filename(parser));
                assert_eq!(
                    diagnostics.len(),
                    test_case.expected_diagnostics.len(),
                    "invalid case {index} ({parser}): {}\n{diagnostics:#?}",
                    test_case.code
                );
                for (diagnostic, expected) in
                    diagnostics.iter().zip(&test_case.expected_diagnostics)
                {
                    assert_eq!(diagnostic.message_id, expected.message_id, "case {index}");
                    assert_eq!(diagnostic.message, expected.message, "case {index}");
                    assert_eq!(diagnostic.data, expected.data, "case {index}");
                    assert_eq!(
                        [diagnostic.range.start, diagnostic.range.end],
                        expected.range,
                        "case {index}"
                    );
                    match (
                        diagnostic
                            .suggestions
                            .first()
                            .and_then(|suggestion| suggestion.fixes.first()),
                        &expected.fix,
                    ) {
                        (Some(actual), Some(expected)) => {
                            assert_eq!(
                                [actual.range.start, actual.range.end],
                                expected.range,
                                "case {index}"
                            );
                            assert_eq!(actual.replacement_text, expected.text, "case {index}");
                        }
                        (None, None) => {}
                        pair => panic!("case {index} fix mismatch: {pair:#?}"),
                    }
                }
                assert_eq!(
                    apply_compatible_fixes(&test_case.code, &diagnostics),
                    test_case.first_pass_output,
                    "first pass case {index} ({parser})"
                );
                assert_eq!(
                    recursively_fixed(&test_case.code, &test_case.options, filename(parser)),
                    test_case.recursive_output,
                    "recursive case {index} ({parser})"
                );
            }
        }
    }

    #[test]
    fn covers_unicode_comments_adjacency_and_nested_source_order() {
        let source = "const marker = '😀日本語'; const view = <><App title={'plain'}>{'outer'}<B>{'inner'}</B>{'tail'}</App></>;";
        let diagnostics = run(source, &json!(["never"]), "fixture.tsx");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [
                "unnecessaryCurly",
                "unnecessaryCurly",
                "unnecessaryCurly",
                "unnecessaryCurly",
            ]
        );
        assert!(
            diagnostics
                .windows(2)
                .all(|pair| pair[0].range.start < pair[1].range.start)
        );
        assert_eq!(
            apply_compatible_fixes(source, &diagnostics),
            Some("const marker = '😀日本語'; const view = <><App title=\"plain\">outer<B>inner</B>tail</App></>;".to_owned())
        );

        for source in [
            "<App>{/* retain */ 'text'}</App>",
            "<App>{'left'}{'right'}</App>",
            "<App>{' '}<B /></App>",
        ] {
            assert!(
                run(source, &json!(["never"]), "fixture.tsx").is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn matches_all_line_terminators_entities_and_escape_guards() {
        for (terminator, expected) in [
            ("\r\n", "<App>{\"before\\r\"}\n{\"after\"}</App>"),
            ("\r", "<App>{\"before\\rafter\"}</App>"),
            ("\n", "<App>{\"before\"}\n{\"after\"}</App>"),
            ("\u{2028}", "<App>{\"before\u{2028}after\"}</App>"),
            ("\u{2029}", "<App>{\"before\u{2029}after\"}</App>"),
        ] {
            let source = format!("<App>before{terminator}after</App>");
            let diagnostics = run(&source, &json!([{ "children": "always" }]), "fixture.tsx");
            assert_eq!(diagnostics.len(), 1, "{terminator:?}");
            assert_eq!(diagnostics[0].message_id, "missingCurly");
            assert_eq!(
                apply_compatible_fixes(&source, &diagnostics).as_deref(),
                Some(expected),
                "{terminator:?}"
            );
        }
        for source in [
            "<App>&nbsp;</App>",
            "<App>{'Hello \\\\n world'}</App>",
            "<App>{'Hello &middot; world'}</App>",
            "<App>{'<Component />'}</App>",
        ] {
            assert!(
                run(source, &json!(["never"]), "fixture.tsx").is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn handles_modes_fragments_elements_and_malformed_inputs() {
        assert!(
            run(
                "<App prop='x'>text</App>",
                &json!(["ignore"]),
                "fixture.tsx"
            )
            .is_empty()
        );
        assert_eq!(
            run(
                "<App prop='x'>text</App>",
                &json!([{ "props": "always", "children": "always" }]),
                "fixture.tsx"
            )
            .len(),
            2
        );
        assert_eq!(
            run(
                "<App>{<>text</>}</App>",
                &json!([{ "children": "never" }]),
                "fixture.tsx"
            )
            .len(),
            1
        );
        assert!(run("<App>{'text'}</App>", &json!([null]), "fixture.tsx").len() == 1);
        assert!(run("<App>{'text'}</App>", &json!([42]), "fixture.tsx").len() == 1);
        assert!(run("<App>{'text'}</App>", &json!(["unknown"]), "fixture.tsx").len() == 1);
        assert!(run("<App>{broken</App>", &json!(["never"]), "fixture.tsx").is_empty());
        assert!(run("const value = 'text';", &json!(["always"]), "fixture.ts").is_empty());
    }
}
