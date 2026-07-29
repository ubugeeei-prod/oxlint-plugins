//! Native implementation of stable `@stylistic/jsx-closing-bracket-location`.
//!
//! Oxc supplies exact JSX opening-element and attribute boundaries. A small
//! UTF-16-aware line map reproduces ESLint columns, including tabs, while
//! a trailing-gap comment scan preserves upstream's fallback and fixes.

use std::{collections::BTreeMap, fmt::Write};

use oxc_allocator::Allocator;
use oxc_ast::ast::JSXOpeningElement;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxlint_plugins_carton::SmallVec;
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-closing-bracket-location";
const MESSAGE_ID: &str = "bracketLocation";

fn joined(prefix: &str, suffix: &str) -> String {
    let mut value = String::with_capacity(prefix.len() + suffix.len());
    value.push_str(prefix);
    value.push_str(suffix);
    value
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Location {
    AfterProps,
    AfterTag,
    PropsAligned,
    TagAligned,
    LineAligned,
    Ignore,
}

impl Location {
    fn from_value(value: &Value) -> Option<Self> {
        if value == &Value::Bool(false) {
            return Some(Self::Ignore);
        }
        match value.as_str()? {
            "after-props" => Some(Self::AfterProps),
            "props-aligned" => Some(Self::PropsAligned),
            "tag-aligned" => Some(Self::TagAligned),
            "line-aligned" => Some(Self::LineAligned),
            _ => None,
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::AfterProps => "placed after the last prop",
            Self::AfterTag => "placed after the opening tag",
            Self::PropsAligned => "aligned with the last prop",
            Self::TagAligned => "aligned with the opening tag",
            Self::LineAligned => "aligned with the line containing the opening tag",
            Self::Ignore => "",
        }
    }
}

#[derive(Clone, Copy)]
struct Options {
    non_empty: Location,
    self_closing: Location,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let config = match options {
            Value::Array(items) => items.first(),
            Value::Null => None,
            value => Some(value),
        };
        if let Some(location) = config.and_then(Location::from_value) {
            return Self {
                non_empty: location,
                self_closing: location,
            };
        }
        let object = config.and_then(Value::as_object);
        if let Some(location) = object
            .and_then(|object| object.get("location"))
            .and_then(Location::from_value)
        {
            return Self {
                non_empty: location,
                self_closing: location,
            };
        }
        Self {
            non_empty: object
                .and_then(|object| object.get("nonEmpty"))
                .and_then(Location::from_value)
                .unwrap_or(Location::TagAligned),
            self_closing: object
                .and_then(|object| object.get("selfClosing"))
                .and_then(Location::from_value)
                .unwrap_or(Location::TagAligned),
        }
    }
}

pub(crate) fn check_jsx_closing_bracket_location(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, options, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, options, diagnostics) {
                break;
            }
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
    let mut visitor = ClosingBracketVisitor {
        source,
        lines: LineMap::new(source),
        options: Options::from_json(options),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct ClosingBracketVisitor<'source, 'diagnostics> {
    source: &'source str,
    lines: LineMap<'source>,
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for ClosingBracketVisitor<'_, '_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        // Upstream listens on `JSXOpeningElement:exit`; preserve nested report
        // order by walking children before checking the current opening tag.
        walk::walk_jsx_opening_element(self, element);
        self.check(element);
    }
}

#[derive(Clone, Copy)]
struct TokenLocations {
    opening: Position,
    opening_start_column: usize,
    tag: Position,
    closing: Position,
    last_prop: Option<PropLocation>,
    self_closing: bool,
}

#[derive(Clone, Copy)]
struct PropLocation {
    span: Span,
    column: usize,
    first_line: usize,
    last_line: usize,
}

#[derive(Clone, Copy)]
struct Position {
    offset: usize,
    line: usize,
    column: usize,
}

impl ClosingBracketVisitor<'_, '_> {
    fn check(&mut self, element: &JSXOpeningElement<'_>) {
        let Some(tokens) = self.locations(element) else {
            return;
        };
        let mut expected = self.expected_location(tokens);
        let same_tab_indentation = expected != Location::TagAligned
            || self.lines.starts_with_tab(tokens.opening.line)
                == self.lines.starts_with_tab(tokens.closing.line);
        let last_boundary = tokens
            .last_prop
            .map_or_else(|| element.name.span().end, |prop| prop.span.end);
        let trailing_comment = usize::try_from(last_boundary)
            .ok()
            .and_then(|boundary| self.last_trailing_comment(boundary, tokens.closing.offset));
        let has_trailing_comment = trailing_comment.is_some();

        if matches!(expected, Location::AfterProps | Location::AfterTag)
            && !(self.has_correct_location(tokens, expected) && same_tab_indentation)
            && has_trailing_comment
        {
            expected = Location::LineAligned;
        }
        if self.has_correct_location(tokens, expected) && same_tab_indentation {
            return;
        }

        let correct_column = self.correct_column(tokens, expected);
        let expected_next_line = correct_column.is_some()
            && tokens
                .last_prop
                .is_some_and(|prop| prop.last_line == tokens.closing.line);
        let details = correct_column.map_or_else(String::new, |column| {
            let mut details = String::from(" (expected column ");
            let _ = write!(details, "{}", column + 1);
            if expected_next_line {
                details.push_str(" on the next line");
            }
            details.push(')');
            details
        });
        let location = expected.description();
        let mut message = String::from("The closing bracket must be ");
        message.push_str(location);
        message.push_str(&details);
        let Some(fix) = self.make_fix(
            element,
            tokens,
            expected,
            correct_column,
            expected_next_line,
            trailing_comment,
        ) else {
            return;
        };
        let Ok(range_start) = u32::try_from(tokens.closing.offset) else {
            return;
        };
        let Some(range_end) = range_start.checked_add(1) else {
            return;
        };
        let range = TextRange::new(range_start, range_end);
        let data = BTreeMap::from([
            ("details".to_owned(), details),
            ("location".to_owned(), location.to_owned()),
        ]);
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: message.clone(),
            data,
            range,
            suggestions: std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message,
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }

    fn locations(&self, element: &JSXOpeningElement<'_>) -> Option<TokenLocations> {
        let start = usize::try_from(element.span.start).ok()?;
        let end = usize::try_from(element.span.end).ok()?;
        let code = self.source.get(start..end)?;
        let self_closing = code.ends_with("/>");
        let closing_offset = end.checked_sub(if self_closing { 2 } else { 1 })?;
        let name_start = usize::try_from(element.name.span().start).ok()?;
        let opening = self.lines.position(start)?;
        let last_prop = element.attributes.last().and_then(|attribute| {
            let span = attribute.span();
            let start = usize::try_from(span.start).ok()?;
            let end = usize::try_from(span.end).ok()?;
            let first = self.lines.position(start)?;
            let last = self.lines.position(end)?;
            Some(PropLocation {
                span,
                column: first.column,
                first_line: first.line,
                last_line: last.line,
            })
        });
        Some(TokenLocations {
            opening,
            opening_start_column: self.lines.leading_column(opening.line),
            tag: self.lines.position(name_start)?,
            closing: self.lines.position(closing_offset)?,
            last_prop,
            self_closing,
        })
    }

    fn expected_location(&self, tokens: TokenLocations) -> Location {
        let Some(last_prop) = tokens.last_prop else {
            return Location::AfterTag;
        };
        if tokens.opening.line == last_prop.last_line {
            return Location::AfterProps;
        }
        if tokens.self_closing {
            self.options.self_closing
        } else {
            self.options.non_empty
        }
    }

    fn has_correct_location(&self, tokens: TokenLocations, expected: Location) -> bool {
        match expected {
            Location::AfterTag => tokens.tag.line == tokens.closing.line,
            Location::AfterProps => tokens
                .last_prop
                .is_some_and(|prop| prop.last_line == tokens.closing.line),
            Location::PropsAligned | Location::TagAligned | Location::LineAligned => self
                .correct_column(tokens, expected)
                .is_some_and(|column| column == tokens.closing.column),
            Location::Ignore => true,
        }
    }

    fn correct_column(&self, tokens: TokenLocations, expected: Location) -> Option<usize> {
        match expected {
            Location::PropsAligned => tokens.last_prop.map(|prop| prop.column),
            Location::TagAligned => Some(tokens.opening.column),
            Location::LineAligned => Some(tokens.opening_start_column),
            Location::AfterProps | Location::AfterTag | Location::Ignore => None,
        }
    }

    fn make_fix(
        &self,
        element: &JSXOpeningElement<'_>,
        tokens: TokenLocations,
        expected: Location,
        correct_column: Option<usize>,
        expected_next_line: bool,
        trailing_comment: Option<Span>,
    ) -> Option<LintFix> {
        let closing_tag = if tokens.self_closing { "/>" } else { ">" };
        let end = element.span.end;
        let (start, replacement) = match expected {
            Location::AfterTag => {
                let start = tokens
                    .last_prop
                    .map_or_else(|| element.name.span().end, |prop| prop.span.end);
                let prefix = if expected_next_line { "\n" } else { " " };
                (start, joined(prefix, closing_tag))
            }
            Location::AfterProps => (
                tokens.last_prop?.span.end,
                joined(if expected_next_line { "\n" } else { "" }, closing_tag),
            ),
            Location::PropsAligned | Location::TagAligned | Location::LineAligned => {
                let start = if let Some(comment) = trailing_comment {
                    comment.end
                } else {
                    tokens.last_prop?.span.end
                };
                let indentation =
                    self.indentation(tokens, expected, correct_column.unwrap_or_default());
                let mut replacement =
                    String::with_capacity(1 + indentation.len() + closing_tag.len());
                replacement.push('\n');
                replacement.push_str(&indentation);
                replacement.push_str(closing_tag);
                (start, replacement)
            }
            Location::Ignore => return None,
        };
        Some(LintFix::replace_range(
            TextRange::new(start, end),
            replacement,
        ))
    }

    fn last_trailing_comment(&self, start: usize, end: usize) -> Option<Span> {
        let mut cursor = start;
        let mut last = None;
        while cursor < end {
            let rest = self.source.get(cursor..end)?;
            if rest.starts_with("//") {
                let length = rest
                    .find(['\r', '\n', '\u{2028}', '\u{2029}'])
                    .unwrap_or(rest.len());
                let comment_end = cursor.checked_add(length)?;
                last = Some(Span::new(
                    u32::try_from(cursor).ok()?,
                    u32::try_from(comment_end).ok()?,
                ));
                cursor = comment_end;
                continue;
            }
            if rest.starts_with("/*") {
                let close = rest.find("*/")?;
                let comment_end = cursor.checked_add(close)?.checked_add(2)?;
                last = Some(Span::new(
                    u32::try_from(cursor).ok()?,
                    u32::try_from(comment_end).ok()?,
                ));
                cursor = comment_end;
                continue;
            }
            let character = rest.chars().next()?;
            cursor = cursor.checked_add(character.len_utf8())?;
        }
        last
    }

    fn indentation(
        &self,
        tokens: TokenLocations,
        expected: Location,
        correct_column: usize,
    ) -> String {
        let line = match expected {
            Location::PropsAligned => tokens
                .last_prop
                .map_or(tokens.opening.line, |prop| prop.first_line),
            Location::TagAligned | Location::LineAligned => tokens.opening.line,
            Location::AfterProps | Location::AfterTag | Location::Ignore => {
                return String::new();
            }
        };
        let mut indentation = self.lines.leading_text(line).to_owned();
        let indentation_column = indentation.encode_utf16().count();
        if correct_column > indentation_column {
            indentation.extend(std::iter::repeat_n(
                ' ',
                correct_column - indentation_column,
            ));
        }
        indentation
    }
}

struct LineMap<'source> {
    source: &'source str,
    starts: SmallVec<[usize; 64]>,
    ends: SmallVec<[usize; 64]>,
}

impl<'source> LineMap<'source> {
    fn new(source: &'source str) -> Self {
        let mut starts = SmallVec::new();
        starts.push(0);
        let mut ends = SmallVec::new();
        let mut characters = source.char_indices().peekable();
        while let Some((offset, character)) = characters.next() {
            match character {
                '\r' => {
                    ends.push(offset);
                    if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                        let _ = characters.next();
                    }
                    let next_start = characters.peek().map_or(source.len(), |(next, _)| *next);
                    starts.push(next_start);
                }
                '\n' | '\u{2028}' | '\u{2029}' => {
                    ends.push(offset);
                    let next_start = characters.peek().map_or(source.len(), |(next, _)| *next);
                    starts.push(next_start);
                }
                _ => {}
            }
        }
        while ends.len() < starts.len() {
            ends.push(source.len());
        }
        Self {
            source,
            starts,
            ends,
        }
    }

    fn position(&self, offset: usize) -> Option<Position> {
        let line_index = self
            .starts
            .partition_point(|start| *start <= offset)
            .checked_sub(1)?;
        let line_start = *self.starts.get(line_index)?;
        Some(Position {
            offset,
            line: line_index + 1,
            column: self.column_between(line_start, offset),
        })
    }

    fn leading_column(&self, line: usize) -> usize {
        self.leading_range(line)
            .map_or(0, |(start, end)| self.column_between(start, end))
    }

    fn leading_text(&self, line: usize) -> &'source str {
        self.leading_range(line)
            .and_then(|(start, end)| self.source.get(start..end))
            .unwrap_or("")
    }

    fn starts_with_tab(&self, line: usize) -> bool {
        line.checked_sub(1)
            .and_then(|index| self.starts.get(index))
            .and_then(|start| self.source.as_bytes().get(*start))
            == Some(&b'\t')
    }

    fn leading_range(&self, line: usize) -> Option<(usize, usize)> {
        let start = *self.starts.get(line.checked_sub(1)?)?;
        let end = *self.ends.get(line - 1)?;
        let text = self.source.get(start..end)?;
        let mut leading_end = start;
        for (relative, character) in text.char_indices() {
            if !character.is_whitespace() && character != '\u{feff}' {
                break;
            }
            leading_end = start + relative + character.len_utf8();
        }
        Some((start, leading_end))
    }

    fn column_between(&self, start: usize, end: usize) -> usize {
        self.source
            .get(start..end)
            .map_or(0, |text| text.encode_utf16().count())
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps JSX option matrices concise"
)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-closing-bracket-location-v5.10.0.json"
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

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_closing_bracket_location(source, Some("fixture.tsx"), &options, &mut diagnostics);
        diagnostics
    }

    fn fixed(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .filter_map(|suggestion| suggestion.fixes.first())
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                usize::try_from(fix.range.start).ok()?..usize::try_from(fix.range.end).ok()?,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    #[test]
    fn replays_every_authored_pinned_upstream_case_exactly() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.valid.len(), 44);
        assert_eq!(fixture.invalid.len(), 65);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .flat_map(|test_case| &test_case.errors)
                .count(),
            65
        );

        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, test_case.options.clone()).is_empty(),
                "valid case {index}: {}",
                test_case.code
            );
        }
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, test_case.options.clone());
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
                assert!(
                    matches!(test_case.code.get(start..end), Some("/" | ">")),
                    "case {index} must report the closing bracket"
                );
                if let (Some(line), Some(column)) = (expected.line, expected.column) {
                    let position = LineMap::new(&test_case.code)
                        .position(start)
                        .expect("diagnostic position");
                    assert_eq!((position.line, position.column + 1), (line, column));
                }
            }
            assert_eq!(
                fixed(&test_case.code, &diagnostics),
                test_case.output,
                "invalid case {index}: {}",
                test_case.code
            );
        }
    }

    #[test]
    fn reports_nested_opening_elements_in_exit_order_and_converges() {
        let source = "<Outer\n  prop={<Inner\n    value />}\n  />";
        let diagnostics = run(source, json!([{ "location": "line-aligned" }]));
        assert_eq!(diagnostics.len(), 2);
        let reported = diagnostics
            .iter()
            .map(|diagnostic| {
                source
                    .get(
                        usize::try_from(diagnostic.range.start).expect("start")
                            ..usize::try_from(diagnostic.range.end).expect("end"),
                    )
                    .expect("reported token")
            })
            .collect::<Vec<_>>();
        assert_eq!(reported, vec!["/", "/"]);
        let output = fixed(source, &diagnostics).expect("fixes");
        assert_eq!(output, "<Outer\n  prop={<Inner\n    value\n  />}\n/>");
        assert!(run(&output, json!([{ "location": "line-aligned" }])).is_empty());
    }

    #[test]
    fn preserves_utf8_byte_ranges_and_utf16_message_columns() {
        let source = "const prefix = \"😀\"; const view = <日本語\n  属性 />;";
        let diagnostics = run(source, json!([{ "location": "tag-aligned" }]));
        assert_eq!(diagnostics.len(), 1);
        let slash = source.rfind('/').expect("slash");
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(slash as u32, slash as u32 + 1)
        );
        assert_eq!(
            diagnostics[0].message,
            "The closing bracket must be aligned with the opening tag (expected column 35 on the next line)"
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some(
                "const prefix = \"😀\"; const view = <日本語\n  属性\n                                  />;"
            )
        );
    }

    #[test]
    fn handles_crlf_cr_and_ecmascript_unicode_line_terminators() {
        for newline in ["\r\n", "\r", "\n", "\u{2028}", "\u{2029}"] {
            let source = format!("<App{newline}  prop />");
            let diagnostics = run(&source, json!([{ "location": "tag-aligned" }]));
            assert_eq!(diagnostics.len(), 1, "{newline:?}");
            assert_eq!(
                fixed(&source, &diagnostics).as_deref(),
                Some(format!("<App{newline}  prop\n/>")).as_deref(),
                "{newline:?}"
            );
        }
    }

    #[test]
    fn honors_independent_non_empty_and_self_closing_disable_switches() {
        let options = json!([{ "nonEmpty": false, "selfClosing": "after-props" }]);
        assert!(run("<App\n  prop\n></App>", options.clone()).is_empty());
        assert_eq!(
            fixed("<App\n  prop\n/>", &run("<App\n  prop\n/>", options)).as_deref(),
            Some("<App\n  prop/>")
        );

        let options = json!([{ "nonEmpty": "after-props", "selfClosing": false }]);
        assert_eq!(
            fixed(
                "<App\n  prop\n></App>",
                &run("<App\n  prop\n></App>", options.clone())
            )
            .as_deref(),
            Some("<App\n  prop></App>")
        );
        assert!(run("<App\n  prop\n/>", options).is_empty());
    }

    #[test]
    fn trailing_comments_fall_back_to_line_alignment_without_deleting_comments() {
        for source in [
            "<App\n  prop\n  // preserve\n  />",
            "<App\n  /* preserve */\n  />",
        ] {
            let diagnostics = run(source, json!([{ "location": "after-props" }]));
            assert_eq!(diagnostics.len(), 1, "{source}");
            assert_eq!(
                diagnostics[0].data.get("location").map(String::as_str),
                Some("aligned with the line containing the opening tag")
            );
            let output = fixed(source, &diagnostics).expect("fix");
            assert!(output.contains("preserve"));
            assert!(output.ends_with("\n/>"));
        }
    }

    #[test]
    fn ignores_non_jsx_invalid_syntax_and_unknown_option_payloads_safely() {
        for source in [
            "const comparison = left < right > value;",
            "const value = '<App\\n/>';",
            "const broken = <App",
        ] {
            assert!(run(source, json!([])).is_empty(), "{source}");
        }
        let diagnostics = run(
            "<App\n  prop />",
            json!([{ "location": "future-mode", "extra": true }]),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].data.get("location").map(String::as_str),
            Some("aligned with the opening tag")
        );
    }
}
