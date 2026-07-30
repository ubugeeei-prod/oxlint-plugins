//! Native implementation of stable `@stylistic/jsx-one-expression-per-line`.
//!
//! The upstream rule groups direct JSX children by their effective content
//! lines. Each child sharing a line with an opening tag, closing tag, or
//! sibling is reported once, with whitespace-preserving replacement details
//! merged when a multiline child conflicts at both ends.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXChild, JSXElement, JSXElementName, JSXFragment};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxlint_plugins_carton::SmallVec;
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-one-expression-per-line";
const MESSAGE_ID: &str = "moveToNewLine";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Allow {
    None,
    Literal,
    SingleChild,
    SingleLine,
    NonJsx,
}

impl Allow {
    fn from_json(options: &Value) -> Self {
        let first = match options {
            Value::Array(values) => values.first(),
            Value::Null => None,
            value => Some(value),
        };
        match first
            .and_then(Value::as_object)
            .and_then(|value| value.get("allow"))
            .and_then(Value::as_str)
        {
            Some("literal") => Self::Literal,
            Some("single-child") => Self::SingleChild,
            Some("single-line") => Self::SingleLine,
            Some("non-jsx") => Self::NonJsx,
            _ => Self::None,
        }
    }
}

pub(crate) fn check_jsx_one_expression_per_line(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allow = Allow::from_json(options);
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, allow, diagnostics);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, allow, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    allow: Allow,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let diagnostic_start = diagnostics.len();
    {
        let mut visitor = JsxOneExpressionPerLine {
            source,
            lines: LineMap::new(source),
            allow,
            diagnostics,
        };
        visitor.visit_program(&parsed.program);
    }
    // ESLint returns reports in source-location order, including reports
    // created by a nested element before a later direct child of its parent.
    diagnostics[diagnostic_start..]
        .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
    true
}

struct JsxOneExpressionPerLine<'source, 'diagnostics> {
    source: &'source str,
    lines: LineMap,
    allow: Allow,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxOneExpressionPerLine<'_, '_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'ast>) {
        if let Some(closing) = &element.closing_element {
            self.check_children(
                &element.children,
                element.opening_element.span,
                closing.span,
            );
        }
        // Upstream listens on JSXElement entry, so parent reports precede
        // diagnostics from nested JSX.
        walk::walk_jsx_element(self, element);
    }

    fn visit_jsx_fragment(&mut self, fragment: &JSXFragment<'ast>) {
        self.check_children(
            &fragment.children,
            fragment.opening_fragment.span,
            fragment.closing_fragment.span,
        );
        walk::walk_jsx_fragment(self, fragment);
    }
}

#[derive(Clone, Copy)]
enum Boundary<'ast> {
    Child(&'ast JSXChild<'ast>),
    Tag(Span),
}

impl Boundary<'_> {
    fn span(self) -> Span {
        match self {
            Self::Child(child) => child.span(),
            Self::Tag(span) => span,
        }
    }
}

struct FixDetails<'ast> {
    child: &'ast JSXChild<'ast>,
    leading_space: bool,
    trailing_space: bool,
    leading_newline: bool,
    trailing_newline: bool,
}

impl<'source> JsxOneExpressionPerLine<'source, '_> {
    fn check_children<'ast>(
        &mut self,
        children: &'ast [JSXChild<'ast>],
        opening: Span,
        closing: Span,
    ) {
        if children.is_empty() {
            return;
        }
        if self.allow == Allow::NonJsx
            && !children
                .iter()
                .any(|child| matches!(child, JSXChild::Element(_) | JSXChild::Fragment(_)))
        {
            return;
        }

        let opening_start_line = self.lines.line(opening.start);
        let opening_end_line = self.lines.line(opening.end);
        let closing_start_line = self.lines.line(closing.start);
        let closing_end_line = self.lines.line(closing.end);

        if children.len() == 1 {
            let child = &children[0];
            let span = child.span();
            let child_start_line = self.lines.line(span.start);
            let child_end_line = self.lines.line(span.end);
            if opening_start_line == opening_end_line
                && opening_end_line == closing_start_line
                && closing_start_line == closing_end_line
                && closing_end_line == child_start_line
                && child_start_line == child_end_line
                && (self.allow == Allow::SingleChild
                    || (self.allow == Allow::Literal && matches!(child, JSXChild::Text(_)))
                    || self.allow == Allow::SingleLine)
            {
                return;
            }
        }

        if self.allow == Allow::SingleLine {
            let first = &children[0];
            let last = &children[children.len() - 1];
            let line_difference = self
                .lines
                .line(last.span().end)
                .saturating_sub(self.lines.line(first.span().start));
            let line_breaks = usize::from(self.text(first).is_some_and(has_leading_lf))
                + usize::from(self.text(last).is_some_and(has_trailing_lf));
            if (line_difference == 0 && line_breaks == 0)
                || (line_difference == 2 && line_breaks == 2)
            {
                return;
            }
        }

        let mut children_by_line = BTreeMap::<usize, Vec<&JSXChild<'ast>>>::new();
        for child in children {
            let span = child.span();
            let mut leading_line = 0;
            let mut trailing_line = 0;
            if let Some(text) = self.text(child) {
                if is_js_whitespace(text) {
                    continue;
                }
                leading_line = usize::from(has_leading_lf(text));
                trailing_line = usize::from(has_trailing_lf(text));
            }

            let start_line = self.lines.line(span.start) + leading_line;
            let end_line = self.lines.line(span.end).saturating_sub(trailing_line);
            children_by_line.entry(start_line).or_default().push(child);
            if start_line != end_line {
                children_by_line.entry(end_line).or_default().push(child);
            }
        }

        if children_by_line.len() == 1 && self.allow == Allow::SingleLine {
            let (&line, grouped) = children_by_line.first_key_value().expect("one line exists");
            let first = grouped[0];
            if line == opening_end_line {
                self.report(first, FixKind::InsertBefore);
            }
            let last = grouped[grouped.len() - 1];
            if line == closing_start_line {
                self.report(last, FixKind::InsertAfter);
            }
            return;
        }

        let mut details = Vec::<FixDetails<'ast>>::new();
        for (&line, grouped) in &children_by_line {
            for (index, child) in grouped.iter().copied().enumerate() {
                let previous = if index == 0 {
                    (line == opening_end_line).then_some(Boundary::Tag(opening))
                } else {
                    Some(Boundary::Child(grouped[index - 1]))
                };
                let next = if index + 1 == grouped.len() && line == closing_start_line {
                    Some(Boundary::Tag(closing))
                } else {
                    None
                };
                if previous.is_none() && next.is_none() {
                    continue;
                }

                let leading_space =
                    previous.is_some_and(|boundary| self.space_between(boundary, child, true));
                let trailing_space =
                    next.is_some_and(|boundary| self.space_between(boundary, child, false));
                let key = child.span().start;
                let detail = if let Some(existing) = details
                    .iter_mut()
                    .find(|detail| detail.child.span().start == key)
                {
                    existing
                } else {
                    details.push(FixDetails {
                        child,
                        leading_space: false,
                        trailing_space: false,
                        leading_newline: false,
                        trailing_newline: false,
                    });
                    details.last_mut().expect("detail was inserted")
                };
                detail.leading_space |= leading_space;
                detail.trailing_space |= trailing_space;
                detail.leading_newline |= previous.is_some();
                detail.trailing_newline |= next.is_some();
            }
        }

        for detail in details {
            self.report(detail.child, FixKind::Replace(detail));
        }
    }

    fn text<'ast>(&self, child: &'ast JSXChild<'ast>) -> Option<&'source str> {
        if !matches!(child, JSXChild::Text(_)) {
            return None;
        }
        self.slice(child.span())
    }

    fn slice(&self, span: Span) -> Option<&'source str> {
        self.source
            .get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
    }

    fn space_between<'ast>(
        &self,
        boundary: Boundary<'ast>,
        child: &'ast JSXChild<'ast>,
        boundary_is_previous: bool,
    ) -> bool {
        let child_text = self.text(child);
        let boundary_text = match boundary {
            Boundary::Child(other) => self.text(other),
            Boundary::Tag(_) => None,
        };
        if boundary_is_previous {
            if boundary_text.is_some_and(|text| text.ends_with(' '))
                || child_text.is_some_and(|text| text.starts_with(' '))
            {
                return true;
            }
        } else if boundary_text.is_some_and(|text| text.starts_with(' '))
            || child_text.is_some_and(|text| text.ends_with(' '))
        {
            return true;
        }

        let boundary_span = boundary.span();
        let child_span = child.span();
        let (start, end) = if boundary_is_previous {
            (boundary_span.end, child_span.start)
        } else {
            (child_span.end, boundary_span.start)
        };
        if start >= end {
            return false;
        }
        self.slice(Span::new(start, end))
            .is_some_and(is_js_whitespace)
    }

    fn report<'ast>(&mut self, child: &'ast JSXChild<'ast>, fix_kind: FixKind<'ast>) {
        let span = child.span();
        let descriptor = self.descriptor(child);
        let mut message = String::with_capacity(descriptor.len() + 37);
        message.push('`');
        message.push_str(&descriptor);
        message.push_str("` must be placed on a new line");
        let range = TextRange::new(span.start, span.end);
        let fix = match fix_kind {
            FixKind::InsertBefore => {
                LintFix::replace_range(TextRange::new(span.start, span.start), "\n")
            }
            FixKind::InsertAfter => {
                LintFix::replace_range(TextRange::new(span.end, span.end), "\n")
            }
            FixKind::Replace(details) => {
                let source = self.slice(span).unwrap_or("").trim_matches(' ');
                let mut replacement = String::with_capacity(source.len() + 16);
                if details.leading_space {
                    replacement.push_str("\n{' '}");
                }
                if details.leading_newline {
                    replacement.push('\n');
                }
                replacement.push_str(source);
                if details.trailing_newline {
                    replacement.push('\n');
                }
                if details.trailing_space {
                    replacement.push_str("{' '}\n");
                }
                LintFix::replace_range(range, replacement)
            }
        };
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: message.clone(),
            data: BTreeMap::from([("descriptor".to_owned(), descriptor)]),
            range,
            suggestions: std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message,
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }

    fn descriptor(&self, child: &JSXChild<'_>) -> String {
        if let JSXChild::Element(element) = child {
            match &element.opening_element.name {
                JSXElementName::Identifier(identifier) => {
                    return identifier.name.as_str().to_owned();
                }
                JSXElementName::IdentifierReference(identifier) => {
                    return identifier.name.as_str().to_owned();
                }
                _ => {}
            }
        }
        self.slice(child.span()).unwrap_or("").replace('\n', "")
    }
}

enum FixKind<'ast> {
    InsertBefore,
    InsertAfter,
    Replace(FixDetails<'ast>),
}

struct LineMap {
    starts: SmallVec<[usize; 64]>,
}

impl LineMap {
    fn new(source: &str) -> Self {
        let mut starts = SmallVec::new();
        starts.push(0);
        let mut characters = source.char_indices().peekable();
        while let Some((_, character)) = characters.next() {
            match character {
                '\r' => {
                    if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                        let _ = characters.next();
                    }
                    starts.push(
                        characters
                            .peek()
                            .map_or(source.len(), |(offset, _)| *offset),
                    );
                }
                '\n' | '\u{2028}' | '\u{2029}' => {
                    starts.push(
                        characters
                            .peek()
                            .map_or(source.len(), |(offset, _)| *offset),
                    );
                }
                _ => {}
            }
        }
        Self { starts }
    }

    fn line(&self, offset: u32) -> usize {
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        self.starts.partition_point(|start| *start <= offset)
    }
}

fn has_leading_lf(text: &str) -> bool {
    text.find('\n')
        .is_some_and(|newline| is_js_whitespace(&text[..newline]))
}

fn has_trailing_lf(text: &str) -> bool {
    text.rfind('\n')
        .is_some_and(|newline| is_js_whitespace(&text[newline + 1..]))
}

fn is_js_whitespace(text: &str) -> bool {
    text.chars().all(|character| {
        matches!(
            character,
            '\t' | '\u{000b}'
                | '\u{000c}'
                | '\r'
                | '\n'
                | ' '
                | '\u{00a0}'
                | '\u{1680}'
                | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    })
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the compatibility matrices concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-one-expression-per-line-v5.10.0.json"
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
        version: String,
        commit: String,
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
        message: Option<String>,
        data: Option<BTreeMap<String, String>>,
    }

    fn run(source: &str, filename: &str, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_one_expression_per_line(source, Some(filename), &options, &mut diagnostics);
        diagnostics
    }

    fn apply_one_pass(source: &str, filename: &str, options: Value) -> String {
        let diagnostics = run(source, filename, options);
        let mut fixes = diagnostics
            .iter()
            .enumerate()
            .filter_map(|(index, diagnostic)| {
                diagnostic
                    .suggestions
                    .first()
                    .and_then(|suggestion| suggestion.fixes.first())
                    .map(|fix| (index, fix))
            })
            .collect::<Vec<_>>();
        fixes.sort_by_key(|(index, fix)| (fix.range.start, fix.range.end, *index));

        // ESLint's SourceCodeFixer rejects fixes that overlap or even touch the
        // previously accepted range. The pinned RuleTester output is one pass.
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
                usize::try_from(fix.range.start).expect("fix start fits usize")
                    ..usize::try_from(fix.range.end).expect("fix end fits usize"),
                &fix.replacement_text,
            );
        }
        output
    }

    #[test]
    fn replays_every_authored_stable_v5_10_0_case_in_jsx_and_tsx() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid JSON");
        assert_eq!(fixture.generated.version, "v5.10.0");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.generated.inventory.valid, 47);
        assert_eq!(fixture.generated.inventory.invalid, 69);
        assert_eq!(fixture.generated.inventory.diagnostics, 84);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 69);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 0);
        assert_eq!(fixture.generated.inventory.total, 116);
        assert_eq!(fixture.valid.len(), 47);
        assert_eq!(fixture.invalid.len(), 69);

        for filename in ["fixture.jsx", "fixture.tsx"] {
            for (index, test_case) in fixture.valid.iter().enumerate() {
                let diagnostics = run(&test_case.code, filename, test_case.options.clone());
                assert!(
                    diagnostics.is_empty(),
                    "{filename} valid fixture {index} reported {diagnostics:#?}\n{}",
                    test_case.code
                );
            }

            for (index, test_case) in fixture.invalid.iter().enumerate() {
                let diagnostics = run(&test_case.code, filename, test_case.options.clone());
                assert_eq!(
                    diagnostics.len(),
                    test_case.errors.len(),
                    "{filename} invalid fixture {index} diagnostic count\n{}",
                    test_case.code
                );
                for (diagnostic, expected) in diagnostics.iter().zip(&test_case.errors) {
                    assert_eq!(diagnostic.rule_name, RULE);
                    assert_eq!(diagnostic.message_id, expected.message_id);
                    let descriptor = diagnostic
                        .data
                        .get("descriptor")
                        .expect("descriptor data exists");
                    assert_eq!(
                        diagnostic.message,
                        format!("`{descriptor}` must be placed on a new line")
                    );
                    if let Some(message) = &expected.message {
                        assert_eq!(&diagnostic.message, message);
                    }
                    if let Some(data) = &expected.data {
                        assert_eq!(&diagnostic.data, data);
                    }
                    assert!(diagnostic.range.start <= diagnostic.range.end);
                    assert!(
                        usize::try_from(diagnostic.range.end)
                            .is_ok_and(|end| end <= test_case.code.len())
                    );
                    assert_eq!(diagnostic.suggestions.len(), 1);
                    assert_eq!(diagnostic.suggestions[0].message_id, MESSAGE_ID);
                    assert_eq!(diagnostic.suggestions[0].message, diagnostic.message);
                    assert_eq!(diagnostic.suggestions[0].fixes.len(), 1);
                }

                let expected_output = test_case
                    .output
                    .as_deref()
                    .expect("all pinned invalid cases are fixable");
                assert_eq!(
                    apply_one_pass(&test_case.code, filename, test_case.options.clone()),
                    expected_output,
                    "{filename} invalid fixture {index} fixed output\n{}",
                    test_case.code
                );
            }
        }
    }

    #[test]
    fn reports_exact_unicode_byte_ranges_messages_data_and_replacements() {
        let source = "const marker = '😀'; const view = <App>日本語<Foo />後</App>;";
        let diagnostics = run(source, "fixture.tsx", Value::Null);
        let text_start = source.find("日本語").expect("text exists");
        let element_start = source.find("<Foo />").expect("element exists");
        let tail_start = source.find('後').expect("tail exists");

        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data["descriptor"].as_str())
                .collect::<Vec<_>>(),
            ["日本語", "Foo", "後"]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [
                TextRange::new(
                    u32::try_from(text_start).unwrap(),
                    u32::try_from(element_start).unwrap()
                ),
                TextRange::new(
                    u32::try_from(element_start).unwrap(),
                    u32::try_from(element_start + "<Foo />".len()).unwrap()
                ),
                TextRange::new(
                    u32::try_from(tail_start).unwrap(),
                    u32::try_from(tail_start + '後'.len_utf8()).unwrap()
                ),
            ]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.suggestions[0].fixes[0].replacement_text.as_str())
                .collect::<Vec<_>>(),
            ["\n日本語", "\n<Foo />", "\n後\n"]
        );
    }

    #[test]
    fn preserves_spaces_fragments_and_eslint_source_report_order() {
        let source = "<App>Hello <><span>inner</span></> tail</App>";
        let diagnostics = run(source, "fixture.jsx", json!([]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data["descriptor"].as_str())
                .collect::<Vec<_>>(),
            [
                "Hello ",
                "<><span>inner</span></>",
                "span",
                "inner",
                " tail"
            ]
        );
        assert_eq!(
            diagnostics[1].suggestions[0].fixes[0].replacement_text,
            "\n{' '}\n<><span>inner</span></>"
        );
        assert_eq!(
            diagnostics[4].suggestions[0].fixes[0].replacement_text,
            "\n{' '}\ntail\n"
        );
    }

    #[test]
    fn honors_every_allow_mode_and_defaults_invalid_options_to_none() {
        let single = "<App>hello</App>";
        for allow in ["literal", "single-child", "single-line", "non-jsx"] {
            assert!(
                run(single, "fixture.tsx", json!([{ "allow": allow }])).is_empty(),
                "{allow}"
            );
        }
        assert_eq!(run(single, "fixture.tsx", json!([])).len(), 1);
        assert_eq!(
            run(single, "fixture.tsx", json!([{ "allow": "unknown" }])).len(),
            1
        );
        assert_eq!(
            run(single, "fixture.tsx", json!([{ "allow": 42 }])).len(),
            1
        );
        assert_eq!(run(single, "fixture.tsx", json!([null])).len(), 1);
        assert_eq!(run(single, "fixture.tsx", json!("invalid")).len(), 1);

        assert!(
            !run(
                "<App>{value}<Child /></App>",
                "fixture.tsx",
                json!([{ "allow": "non-jsx" }])
            )
            .is_empty()
        );
        assert!(
            run(
                "<App>{<Child />}</App>",
                "fixture.tsx",
                json!([{ "allow": "non-jsx" }])
            )
            .is_empty()
        );
    }

    #[test]
    fn uses_lf_fixes_with_crlf_and_handles_all_ecmascript_line_terminators() {
        let source = "<App>\r\n  日本語<Foo />\r\n</App>";
        let diagnostics = run(source, "fixture.tsx", Value::Null);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data["descriptor"].as_str())
                .collect::<Vec<_>>(),
            ["Foo"]
        );
        let element_start = u32::try_from(source.find("<Foo />").unwrap()).unwrap();
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(
                element_start,
                element_start + u32::try_from("<Foo />".len()).unwrap()
            )
        );
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0],
            LintFix::replace_range(diagnostics[0].range, "\n<Foo />")
        );

        for terminator in ["\r", "\u{2028}", "\u{2029}"] {
            let source = format!("<App>{terminator}<Foo />{terminator}</App>");
            assert!(
                run(&source, "fixture.tsx", Value::Null).is_empty(),
                "{terminator:?}"
            );
        }
    }

    #[test]
    fn handles_single_line_insertions_parse_failures_and_filename_source_types() {
        let opening = run(
            "<App\n  foo\n>Up to {percent}% Off\n</App>",
            "fixture.tsx",
            json!([
                { "allow": "single-line" }
            ]),
        );
        assert_eq!(opening.len(), 1);
        let opening_fix = &opening[0].suggestions[0].fixes[0];
        assert_eq!(opening_fix.range.start, opening[0].range.start);
        assert_eq!(opening_fix.range.start, opening_fix.range.end);
        assert_eq!(opening_fix.replacement_text, "\n");

        let closing = run(
            "<App\n  foo\n>\n  Up to {percent}% Off</App>",
            "fixture.tsx",
            json!([
                { "allow": "single-line" }
            ]),
        );
        assert_eq!(closing.len(), 1);
        let closing_fix = &closing[0].suggestions[0].fixes[0];
        assert_eq!(closing_fix.range.start, closing[0].range.end);
        assert_eq!(closing_fix.range.start, closing_fix.range.end);
        assert_eq!(closing_fix.replacement_text, "\n");

        assert!(run("<App><Broken></App>", "fixture.tsx", Value::Null).is_empty());
        assert!(run("<App>text</App>", "fixture.ts", Value::Null).is_empty());
        assert_eq!(run("<App>text</App>", "fixture.jsx", Value::Null).len(), 1);
        assert_eq!(run("<App>text</App>", "fixture.tsx", Value::Null).len(), 1);
    }
}
