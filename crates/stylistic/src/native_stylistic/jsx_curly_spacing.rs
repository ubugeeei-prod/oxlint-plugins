//! Native implementation of stable `@stylistic/jsx-curly-spacing`.
//!
//! Oxc's JSX AST identifies expression containers, spread attributes, and
//! their exact parent context. Boundary-only comment scanning then reproduces
//! ESLint's comment-inclusive token queries without treating JSX text as
//! JavaScript. Fixes retain comments and implement the upstream JavaScript
//! `/^\s+/gm` and `/\s+$/gm` replacement semantics.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    JSXElement, JSXExpression, JSXExpressionContainer, JSXFragment, JSXOpeningElement,
    JSXSpreadAttribute,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-curly-spacing";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Spacing {
    Always,
    Never,
}

impl Spacing {
    fn from_value(value: Option<&Value>) -> Option<Self> {
        match value.and_then(Value::as_str) {
            Some("always") => Some(Self::Always),
            Some("never") => Some(Self::Never),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BasicConfig {
    when: Spacing,
    allow_multiline: bool,
    object_literal_spaces: Spacing,
}

#[derive(Clone, Copy, Debug)]
struct Options {
    attributes: Option<BasicConfig>,
    children: Option<BasicConfig>,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let items = options.as_array();
        let first = items.and_then(|items| items.first());
        let second = items
            .and_then(|items| items.get(1))
            .and_then(Value::as_object);

        if let Some(default_when) = Spacing::from_value(first) {
            let default_allow_multiline = second
                .and_then(|object| object.get("allowMultiline"))
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let default_object_literal_spaces = second
                .and_then(|object| object.get("spacing"))
                .and_then(Value::as_object)
                .and_then(|spacing| Spacing::from_value(spacing.get("objectLiterals")));
            return Self {
                attributes: Some(BasicConfig {
                    when: default_when,
                    allow_multiline: default_allow_multiline,
                    object_literal_spaces: default_object_literal_spaces.unwrap_or(default_when),
                }),
                children: None,
            };
        }

        let object = first.and_then(Value::as_object);
        let default_when = object
            .and_then(|object| Spacing::from_value(object.get("when")))
            .unwrap_or(Spacing::Never);
        let default_allow_multiline = object
            .and_then(|object| object.get("allowMultiline"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let default_object_literal_spaces = object
            .and_then(|object| object.get("spacing"))
            .and_then(Value::as_object)
            .and_then(|spacing| Spacing::from_value(spacing.get("objectLiterals")));

        Self {
            attributes: normalize_scope(
                object.and_then(|object| object.get("attributes")),
                true,
                default_when,
                default_allow_multiline,
                default_object_literal_spaces,
            ),
            children: normalize_scope(
                object.and_then(|object| object.get("children")),
                false,
                default_when,
                default_allow_multiline,
                default_object_literal_spaces,
            ),
        }
    }
}

fn normalize_scope(
    value: Option<&Value>,
    enabled_by_default: bool,
    default_when: Spacing,
    default_allow_multiline: bool,
    default_object_literal_spaces: Option<Spacing>,
) -> Option<BasicConfig> {
    match value {
        Some(Value::Bool(false)) => None,
        None if !enabled_by_default => None,
        Some(Value::Object(object)) => {
            let when = Spacing::from_value(object.get("when")).unwrap_or(default_when);
            let allow_multiline = object
                .get("allowMultiline")
                .and_then(Value::as_bool)
                .unwrap_or(default_allow_multiline);
            let inherited_spacing =
                object
                    .get("spacing")
                    .map_or(default_object_literal_spaces, |spacing| {
                        spacing
                            .as_object()
                            .and_then(|spacing| Spacing::from_value(spacing.get("objectLiterals")))
                    });
            Some(BasicConfig {
                when,
                allow_multiline,
                object_literal_spaces: inherited_spacing.unwrap_or(when),
            })
        }
        Some(Value::Bool(true)) | None => Some(BasicConfig {
            when: default_when,
            allow_multiline: default_allow_multiline,
            object_literal_spaces: default_object_literal_spaces.unwrap_or(default_when),
        }),
        Some(_) => None,
    }
}

pub(crate) fn check_jsx_curly_spacing(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
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
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = JsxCurlySpacing {
        source,
        options: Options::from_json(options),
        contexts: Vec::new(),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

#[derive(Clone, Copy)]
enum Context {
    Attributes,
    Children,
}

struct JsxCurlySpacing<'source, 'diagnostics> {
    source: &'source str,
    options: Options,
    contexts: Vec<Context>,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxCurlySpacing<'_, '_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'ast>) {
        self.contexts.push(Context::Children);
        walk::walk_jsx_element(self, element);
        self.contexts.pop();
    }

    fn visit_jsx_fragment(&mut self, fragment: &JSXFragment<'ast>) {
        self.contexts.push(Context::Children);
        walk::walk_jsx_fragment(self, fragment);
        self.contexts.pop();
    }

    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        self.contexts.push(Context::Attributes);
        walk::walk_jsx_opening_element(self, element);
        self.contexts.pop();
    }

    fn visit_jsx_expression_container(&mut self, container: &JSXExpressionContainer<'ast>) {
        let config = match self.contexts.last() {
            Some(Context::Attributes) => self.options.attributes,
            Some(Context::Children) => self.options.children,
            None => None,
        };
        if let Some(config) = config {
            let inner = match &container.expression {
                JSXExpression::EmptyExpression(_) => Inner::Empty,
                expression => Inner::Expression(expression.span()),
            };
            self.check(container.span, inner, config);
        }
        walk::walk_jsx_expression_container(self, container);
    }

    fn visit_jsx_spread_attribute(&mut self, attribute: &JSXSpreadAttribute<'ast>) {
        if let Some(config) = self.options.attributes {
            self.check(
                attribute.span,
                Inner::Spread(attribute.argument.span()),
                config,
            );
        }
        walk::walk_jsx_spread_attribute(self, attribute);
    }
}

#[derive(Clone, Copy)]
enum Inner {
    Empty,
    Expression(Span),
    Spread(Span),
}

#[derive(Clone, Copy, Debug)]
struct BoundaryToken {
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct Boundaries {
    open: BoundaryToken,
    close: BoundaryToken,
    second: BoundaryToken,
    penultimate: BoundaryToken,
    next_significant: BoundaryToken,
    previous_significant: BoundaryToken,
    leading_comments: Vec<BoundaryToken>,
    trailing_comments: Vec<BoundaryToken>,
}

impl JsxCurlySpacing<'_, '_> {
    fn check(&mut self, span: Span, inner: Inner, config: BasicConfig) {
        let Some(tokens) = Boundaries::new(self.source, span, inner) else {
            return;
        };
        let is_object_literal = self
            .source
            .get(tokens.second.start..tokens.second.end)
            .is_some_and(|text| text == "{");
        let spacing = if is_object_literal {
            config.object_literal_spaces
        } else {
            config.when
        };

        match spacing {
            Spacing::Always => {
                if !has_space_between(self.source, tokens.open, tokens.second) {
                    self.report(
                        tokens.open,
                        "spaceNeededAfter",
                        "A space is required after '{'",
                        LintFix::replace_range(byte_range(tokens.open.end, tokens.open.end), " "),
                    );
                } else if !config.allow_multiline
                    && !tokens_on_same_line(self.source, tokens.open, tokens.second)
                {
                    let replacement = trim_start(
                        self.source
                            .get(tokens.open.end..tokens.next_significant.start)
                            .unwrap_or_default(),
                    ) + " ";
                    self.report(
                        tokens.open,
                        "noNewlineAfter",
                        "There should be no newline after '{'",
                        LintFix::replace_range(
                            byte_range(tokens.open.end, tokens.next_significant.start),
                            replacement,
                        ),
                    );
                }

                if !has_space_between(self.source, tokens.penultimate, tokens.close) {
                    self.report(
                        tokens.close,
                        "spaceNeededBefore",
                        "A space is required before '}'",
                        LintFix::replace_range(
                            byte_range(tokens.close.start, tokens.close.start),
                            " ",
                        ),
                    );
                } else if !config.allow_multiline
                    && !tokens_on_same_line(self.source, tokens.penultimate, tokens.close)
                {
                    let replacement = " ".to_owned()
                        + &trim_end(
                            self.source
                                .get(tokens.previous_significant.end..tokens.close.start)
                                .unwrap_or_default(),
                        );
                    self.report(
                        tokens.close,
                        "noNewlineBefore",
                        "There should be no newline before '}'",
                        LintFix::replace_range(
                            byte_range(tokens.previous_significant.end, tokens.close.start),
                            replacement,
                        ),
                    );
                }
            }
            Spacing::Never => {
                if !tokens_on_same_line(self.source, tokens.open, tokens.second) {
                    if !config.allow_multiline {
                        let replacement = trim_start(
                            self.source
                                .get(tokens.open.end..tokens.next_significant.start)
                                .unwrap_or_default(),
                        );
                        self.report(
                            tokens.open,
                            "noNewlineAfter",
                            "There should be no newline after '{'",
                            LintFix::replace_range(
                                byte_range(tokens.open.end, tokens.next_significant.start),
                                replacement,
                            ),
                        );
                    }
                } else if has_space_between(self.source, tokens.open, tokens.second) {
                    let end = tokens
                        .leading_comments
                        .first()
                        .map_or(tokens.next_significant.start, |comment| comment.start);
                    let replacement =
                        trim_start(self.source.get(tokens.open.end..end).unwrap_or_default());
                    self.report(
                        tokens.open,
                        "noSpaceAfter",
                        "There should be no space after '{'",
                        LintFix::replace_range(byte_range(tokens.open.end, end), replacement),
                    );
                }

                if !tokens_on_same_line(self.source, tokens.penultimate, tokens.close) {
                    if !config.allow_multiline {
                        let replacement = trim_end(
                            self.source
                                .get(tokens.previous_significant.end..tokens.close.start)
                                .unwrap_or_default(),
                        );
                        self.report(
                            tokens.close,
                            "noNewlineBefore",
                            "There should be no newline before '}'",
                            LintFix::replace_range(
                                byte_range(tokens.previous_significant.end, tokens.close.start),
                                replacement,
                            ),
                        );
                    }
                } else if has_space_between(self.source, tokens.penultimate, tokens.close) {
                    let start = tokens
                        .trailing_comments
                        .first()
                        .map_or(tokens.previous_significant.end, |comment| {
                            tokens.previous_significant.end.max(comment.end)
                        });
                    let replacement = trim_end(
                        self.source
                            .get(start..tokens.close.start)
                            .unwrap_or_default(),
                    );
                    self.report(
                        tokens.close,
                        "noSpaceBefore",
                        "There should be no space before '}'",
                        LintFix::replace_range(byte_range(start, tokens.close.start), replacement),
                    );
                }
            }
        }
    }

    fn report(&mut self, token: BoundaryToken, message_id: &str, message: &str, fix: LintFix) {
        let brace = self.source.get(token.start..token.end).unwrap_or_default();
        let data = BTreeMap::from([("token".to_owned(), brace.to_owned())]);
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data,
            range: byte_range(token.start, token.end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }
}

impl Boundaries {
    fn new(source: &str, span: Span, inner: Inner) -> Option<Self> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let close_start = end.checked_sub(1)?;
        if source.get(start..start + 1)? != "{" || source.get(close_start..end)? != "}" {
            return None;
        }
        let open = BoundaryToken {
            start,
            end: start + 1,
        };
        let close = BoundaryToken {
            start: close_start,
            end,
        };

        let (core_start, core_end) = match inner {
            Inner::Empty => (None, None),
            Inner::Expression(span) => (
                Some(usize::try_from(span.start).ok()?),
                Some(usize::try_from(span.end).ok()?),
            ),
            Inner::Spread(argument) => {
                let argument_start = usize::try_from(argument.start).ok()?;
                let spread = first_non_comment(source, open.end, argument_start)?;
                if source.get(spread..spread.checked_add(3)?)? != "..." {
                    return None;
                }
                (Some(spread), Some(usize::try_from(argument.end).ok()?))
            }
        };

        if let (Some(core_start), Some(core_end)) = (core_start, core_end) {
            if !(open.end <= core_start && core_start < core_end && core_end <= close.start) {
                return None;
            }
            let leading_comments = comments_in_gap(source, open.end, core_start);
            let trailing_comments = comments_in_gap(source, core_end, close.start);
            let core_first = BoundaryToken {
                start: core_start,
                end: if source.get(core_start..core_start + 1)? == "{" {
                    core_start + 1
                } else {
                    core_start
                        + source
                            .get(core_start..)?
                            .chars()
                            .next()
                            .map_or(1, char::len_utf8)
                },
            };
            let core_last = BoundaryToken {
                start: core_end.saturating_sub(1),
                end: core_end,
            };
            Some(Self {
                open,
                close,
                second: leading_comments.first().copied().unwrap_or(core_first),
                penultimate: trailing_comments.last().copied().unwrap_or(core_last),
                next_significant: core_first,
                previous_significant: core_last,
                leading_comments,
                trailing_comments,
            })
        } else {
            let comments = comments_in_gap(source, open.end, close.start);
            Some(Self {
                open,
                close,
                second: comments.first().copied().unwrap_or(close),
                penultimate: comments.last().copied().unwrap_or(open),
                next_significant: close,
                previous_significant: open,
                leading_comments: comments.clone(),
                trailing_comments: comments,
            })
        }
    }
}

fn comments_in_gap(source: &str, start: usize, end: usize) -> Vec<BoundaryToken> {
    let mut comments = Vec::new();
    let mut position = start;
    while position < end {
        position = skip_whitespace(source, position, end);
        if position >= end {
            break;
        }
        let bytes = source.as_bytes();
        if bytes.get(position..position + 2) == Some(b"//") {
            let comment_start = position;
            position += 2;
            while position < end {
                let Some(character) = source
                    .get(position..end)
                    .and_then(|text| text.chars().next())
                else {
                    break;
                };
                if is_line_terminator(character) {
                    break;
                }
                position += character.len_utf8();
            }
            comments.push(BoundaryToken {
                start: comment_start,
                end: position,
            });
        } else if bytes.get(position..position + 2) == Some(b"/*") {
            let comment_start = position;
            position = source
                .get(position + 2..end)
                .and_then(|text| text.find("*/"))
                .map_or(end, |relative| position + 2 + relative + 2);
            comments.push(BoundaryToken {
                start: comment_start,
                end: position,
            });
        } else {
            break;
        }
    }
    comments
}

fn first_non_comment(source: &str, start: usize, end: usize) -> Option<usize> {
    let comments = comments_in_gap(source, start, end);
    let after_comments = comments.last().map_or(start, |comment| comment.end);
    Some(skip_whitespace(source, after_comments, end))
}

fn skip_whitespace(source: &str, mut position: usize, end: usize) -> usize {
    while position < end {
        let Some(character) = source
            .get(position..end)
            .and_then(|text| text.chars().next())
        else {
            break;
        };
        if !is_ecmascript_whitespace(character) {
            break;
        }
        position += character.len_utf8();
    }
    position
}

fn has_space_between(source: &str, left: BoundaryToken, right: BoundaryToken) -> bool {
    source
        .get(left.end..right.start)
        .is_some_and(|gap| gap.chars().any(is_ecmascript_whitespace))
}

fn tokens_on_same_line(source: &str, left: BoundaryToken, right: BoundaryToken) -> bool {
    source
        .get(left.end..right.start)
        .is_some_and(|gap| !gap.chars().any(is_line_terminator))
}

fn trim_start(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut position = 0;
    let mut candidate = Some(0);
    while position < text.len() {
        if candidate == Some(position) {
            let end = skip_whitespace(text, position, text.len());
            if end > position {
                position = end;
                candidate = None;
                continue;
            }
        }
        let Some(character) = text.get(position..).and_then(|rest| rest.chars().next()) else {
            break;
        };
        output.push(character);
        position += character.len_utf8();
        candidate = is_line_terminator(character).then_some(position);
    }
    output
}

fn trim_end(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut position = 0;
    while position < text.len() {
        let Some(character) = text.get(position..).and_then(|rest| rest.chars().next()) else {
            break;
        };
        if !is_ecmascript_whitespace(character) {
            output.push(character);
            position += character.len_utf8();
            continue;
        }

        let run_start = position;
        let mut run_end = position;
        let mut last_terminator = None;
        while run_end < text.len() {
            let Some(character) = text.get(run_end..).and_then(|rest| rest.chars().next()) else {
                break;
            };
            if !is_ecmascript_whitespace(character) {
                break;
            }
            if is_line_terminator(character) {
                last_terminator = Some(run_end);
            }
            run_end += character.len_utf8();
        }

        if run_end == text.len() {
            break;
        }
        if let Some(terminator) = last_terminator {
            output.push_str(&text[terminator..run_end]);
        } else {
            output.push_str(&text[run_start..run_end]);
        }
        position = run_end;
    }
    output
}

fn is_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
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

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the focused JSX option matrices concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-curly-spacing-v5.10.0.json"
    ));

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
    }

    #[derive(Deserialize)]
    struct TestCase {
        code: String,
        #[serde(default)]
        options: Value,
        output: Option<String>,
        #[serde(default)]
        errors: Vec<ExpectedError>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedError {
        message_id: String,
        message: String,
        data: BTreeMap<String, String>,
        line: Option<usize>,
        column: Option<usize>,
    }

    fn run(source: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_curly_spacing(source, Some("fixture.tsx"), options, &mut diagnostics);
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
                usize::try_from(fix.range.start).ok()?..usize::try_from(fix.range.end).ok()?,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    fn recursively_fixed(source: &str, options: &Value) -> String {
        let mut output = source.to_owned();
        for _ in 0..10 {
            let diagnostics = run(&output, options);
            if diagnostics.is_empty() {
                return output;
            }
            let Some(next) = apply_compatible_fixes(&output, &diagnostics) else {
                return output;
            };
            assert_ne!(next, output, "a fix pass must make progress");
            output = next;
        }
        output
    }

    #[test]
    fn replays_all_296_authored_pinned_upstream_cases() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.valid.len(), 142);
        assert_eq!(fixture.invalid.len(), 154);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .flat_map(|test_case| &test_case.errors)
                .count(),
            320
        );

        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, &test_case.options).is_empty(),
                "valid case {index}: {}\n{:#?}",
                test_case.code,
                run(&test_case.code, &test_case.options)
            );
        }
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, &test_case.options);
            assert_eq!(
                diagnostics.len(),
                test_case.errors.len(),
                "invalid case {index}: {}",
                test_case.code
            );
            for (diagnostic, expected) in diagnostics.iter().zip(&test_case.errors) {
                assert_eq!(diagnostic.message_id, expected.message_id, "case {index}");
                assert_eq!(diagnostic.message, expected.message, "case {index}");
                assert_eq!(diagnostic.data, expected.data, "case {index}");
                let start = usize::try_from(diagnostic.range.start).expect("range start");
                let end = usize::try_from(diagnostic.range.end).expect("range end");
                assert_eq!(
                    test_case.code.get(start..end),
                    expected.data.get("token").map(String::as_str),
                    "case {index}"
                );
                if let (Some(line), Some(column)) = (expected.line, expected.column) {
                    assert_eq!(
                        location_at(&test_case.code, start),
                        (line, column),
                        "case {index}"
                    );
                }
                assert_eq!(diagnostic.suggestions.len(), 1, "case {index}");
            }
            assert_eq!(
                recursively_fixed(&test_case.code, &test_case.options),
                test_case.output.as_deref().unwrap_or(&test_case.code),
                "invalid case {index}: {}",
                test_case.code
            );
        }
    }

    #[test]
    fn separates_attribute_child_spread_and_nested_contexts() {
        let source =
            "<Outer attr={value} {...props}>{child}<Inner attr={other}>{nested}</Inner></Outer>";
        let diagnostics = run(
            source,
            &json!([{
                "attributes": { "when": "always" },
                "children": { "when": "always" }
            }]),
        );
        assert_eq!(diagnostics.len(), 10);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [
                "spaceNeededAfter",
                "spaceNeededBefore",
                "spaceNeededAfter",
                "spaceNeededBefore",
                "spaceNeededAfter",
                "spaceNeededBefore",
                "spaceNeededAfter",
                "spaceNeededBefore",
                "spaceNeededAfter",
                "spaceNeededBefore",
            ]
        );
        let output = recursively_fixed(
            source,
            &json!([{
                "attributes": { "when": "always" },
                "children": { "when": "always" }
            }]),
        );
        assert_eq!(
            output,
            "<Outer attr={ value } { ...props }>{ child }<Inner attr={ other }>{ nested }</Inner></Outer>"
        );
    }

    #[test]
    fn preserves_comments_and_matches_javascript_multiline_trimming() {
        assert_eq!(trim_start("\n  foo"), "foo");
        assert_eq!(trim_start("x\n  foo"), "x\nfoo");
        assert_eq!(trim_start("x\r\n  foo"), "x\rfoo");
        assert_eq!(trim_start("x\u{2028}  foo"), "x\u{2028}foo");
        assert_eq!(trim_end("\n  foo"), "\n  foo");
        assert_eq!(trim_end("x  \n  foo"), "x\n  foo");
        assert_eq!(trim_end("x\r\n  foo"), "x\n  foo");
        assert_eq!(trim_end(" \n x \n "), "\n x");

        let source = "<App foo={\n /* first */\n value\n /* last */\n} />";
        let options = json!([{ "attributes": { "when": "always", "allowMultiline": false } }]);
        let diagnostics = run(source, &options);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["noNewlineAfter", "noNewlineBefore"]
        );
        assert_eq!(
            recursively_fixed(source, &options),
            "<App foo={ /* first */\n value \n /* last */ } />"
        );
    }

    #[test]
    fn honors_object_literal_override_empty_comments_and_disable_switches() {
        let options = json!([{
            "when": "never",
            "spacing": { "objectLiterals": "always" },
            "attributes": true,
            "children": true
        }]);
        let source = "<><App object={{value: 1}} plain={ value }>{/* keep */}{{child: 1}}</App></>";
        assert_eq!(
            recursively_fixed(source, &options),
            "<><App object={ {value: 1} } plain={value}>{/* keep */}{ {child: 1} }</App></>"
        );
        assert!(
            run(
                "<App attr={ spaced }>{ spaced }</App>",
                &json!([{ "attributes": false, "children": false }])
            )
            .is_empty()
        );
    }

    #[test]
    fn reports_utf8_byte_ranges_that_map_to_utf16_brace_locations() {
        let source = "const emoji = '😀'; const view = <App attr={value}>{child}</App>;";
        let diagnostics = run(
            source,
            &json!([{ "attributes": { "when": "always" }, "children": { "when": "always" } }]),
        );
        assert_eq!(diagnostics.len(), 4);
        let brace_offsets = source
            .match_indices(['{', '}'])
            .map(|(offset, _)| offset)
            .collect::<Vec<_>>();
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| usize::try_from(diagnostic.range.start).expect("start"))
                .collect::<Vec<_>>(),
            brace_offsets
        );
        for diagnostic in &diagnostics {
            let start = usize::try_from(diagnostic.range.start).expect("start");
            let utf16 = source[..start].encode_utf16().count();
            assert!(utf16 < start);
            assert_eq!(diagnostic.range.end, diagnostic.range.start + 1);
        }
    }

    #[test]
    fn supports_jsx_tsx_filenames_and_silently_ignores_non_jsx_or_invalid_syntax() {
        let options = json!([{ "when": "always", "children": true }]);
        for filename in ["fixture.jsx", "fixture.tsx"] {
            let mut diagnostics = Vec::new();
            check_jsx_curly_spacing(
                "<App prop={value}>{child}</App>",
                Some(filename),
                &options,
                &mut diagnostics,
            );
            assert_eq!(diagnostics.len(), 4, "{filename}");
        }
        for (source, filename) in [
            ("const value = { plain: true };", "fixture.ts"),
            ("<App prop={value>", "fixture.tsx"),
        ] {
            let mut diagnostics = Vec::new();
            check_jsx_curly_spacing(source, Some(filename), &options, &mut diagnostics);
            assert!(diagnostics.is_empty(), "{source}");
        }
    }

    fn location_at(source: &str, offset: usize) -> (usize, usize) {
        let prefix = &source[..offset];
        let mut line = 1;
        let mut line_start = 0;
        let mut chars = prefix.char_indices().peekable();
        while let Some((index, character)) = chars.next() {
            if character == '\r' {
                if chars.peek().is_some_and(|(_, next)| *next == '\n') {
                    let _ = chars.next();
                }
                line += 1;
                line_start = index + character.len_utf8();
            } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
                line += 1;
                line_start = index + character.len_utf8();
            }
        }
        (line, source[line_start..offset].encode_utf16().count() + 1)
    }
}
