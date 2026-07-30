//! Native implementation of stable
//! `@stylistic/jsx-props-no-multi-spaces`.
//!
//! Oxc supplies exact opening-name, generic, attribute, spread, and comment
//! boundaries. The rule intentionally preserves upstream's two independent
//! checks: blank lines are unfixable, while every non-single-space inline gap
//! is replaced wholesale (including an inline comment).

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    Comment, CommentPosition,
    ast::{JSXAttributeItem, JSXOpeningElement},
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-props-no-multi-spaces";
const NO_LINE_GAP_ID: &str = "noLineGap";
const ONLY_ONE_SPACE_ID: &str = "onlyOneSpace";

pub(crate) fn check_jsx_props_no_multi_spaces(
    source: &str,
    filename: Option<&str>,
    _options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok())
        && parse_and_check(source, source_type, diagnostics)
    {
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = PropsSpacingVisitor {
        source,
        comments: &parsed.program.comments,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct PropsSpacingVisitor<'source, 'comments, 'diagnostics> {
    source: &'source str,
    comments: &'comments [Comment],
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for PropsSpacingVisitor<'_, '_, '_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        // Upstream listens on entry, so all reports for an outer opening tag
        // precede reports produced by JSX nested inside one of its values.
        self.check(element);
        walk::walk_jsx_opening_element(self, element);
    }
}

impl PropsSpacingVisitor<'_, '_, '_> {
    fn check(&mut self, element: &JSXOpeningElement<'_>) {
        let name_span = element.name.span();
        let mut previous_span = element
            .type_arguments
            .as_ref()
            .map_or(name_span, |type_arguments| {
                Span::new(name_span.start, type_arguments.span.end)
            });
        let Some(mut previous_name) = self.source_slice(name_span).map(str::to_owned) else {
            return;
        };

        for attribute in &element.attributes {
            let current_span = attribute.span();
            let Some(current_name) = self.attribute_name(attribute) else {
                previous_span = current_span;
                continue;
            };
            self.check_gap(previous_span, &previous_name, current_span, &current_name);
            previous_span = current_span;
            previous_name = current_name;
        }
    }

    fn attribute_name(&self, attribute: &JSXAttributeItem<'_>) -> Option<String> {
        match attribute {
            JSXAttributeItem::Attribute(attribute) => {
                self.source_slice(attribute.name.span()).map(str::to_owned)
            }
            JSXAttributeItem::SpreadAttribute(attribute) => self
                .source_slice(attribute.argument.span())
                .map(str::to_owned),
        }
    }

    fn check_gap(
        &mut self,
        previous: Span,
        previous_name: &str,
        current: Span,
        current_name: &str,
    ) {
        if self.has_empty_line(previous, current) {
            self.report(
                NO_LINE_GAP_ID,
                "Expected no line gap between",
                previous_name,
                current_name,
                current,
                None,
            );
        }

        let previous_end = usize::try_from(previous.end).ok();
        let current_start = usize::try_from(current.start).ok();
        let current_end = usize::try_from(current.end).ok();
        let (Some(previous_end), Some(current_start), Some(current_end)) =
            (previous_end, current_start, current_end)
        else {
            return;
        };
        if previous_end > current_start
            || current_end > self.source.len()
            || line_at(self.source, previous_end) != line_at(self.source, current_end)
            || self.source.get(previous_end..current_start) == Some(" ")
        {
            return;
        }

        self.report(
            ONLY_ONE_SPACE_ID,
            "Expected only one space between",
            previous_name,
            current_name,
            current,
            Some(LintFix::replace_range(
                byte_range(previous_end, current_start),
                " ",
            )),
        );
    }

    fn has_empty_line(&self, previous: Span, current: Span) -> bool {
        let mut previous_end = previous.end;
        for comment in self.comments.iter().filter(|comment| {
            comment.position == CommentPosition::Leading
                && comment.attached_to == current.start
                && comment.span.start >= previous.end
                && comment.span.end <= current.start
        }) {
            if line_at_u32(self.source, comment.span.start)
                .saturating_sub(line_at_u32(self.source, previous_end))
                >= 2
            {
                return true;
            }
            previous_end = comment.span.end;
        }

        line_at_u32(self.source, current.start)
            .saturating_sub(line_at_u32(self.source, previous_end))
            >= 2
    }

    fn report(
        &mut self,
        message_id: &str,
        prefix: &str,
        previous_name: &str,
        current_name: &str,
        current: Span,
        fix: Option<LintFix>,
    ) {
        let mut message =
            String::with_capacity(prefix.len() + previous_name.len() + current_name.len() + 11);
        message.push_str(prefix);
        message.push_str(" “");
        message.push_str(previous_name);
        message.push_str("” and “");
        message.push_str(current_name);
        message.push('”');
        let data = BTreeMap::from([
            ("prop1".to_owned(), previous_name.to_owned()),
            ("prop2".to_owned(), current_name.to_owned()),
        ]);
        let suggestions = fix.map_or_else(Vec::new, |fix| {
            std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.clone(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect()
        });
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message,
            data,
            range: TextRange::new(current.start, current.end),
            suggestions,
        });
    }

    fn source_slice(&self, span: Span) -> Option<&str> {
        self.source
            .get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
    }
}

fn line_at_u32(source: &str, offset: u32) -> usize {
    usize::try_from(offset).map_or(usize::MAX, |offset| line_at(source, offset))
}

fn line_at(source: &str, offset: usize) -> usize {
    let Some(prefix) = source.get(..offset) else {
        return usize::MAX;
    };
    let mut lines = 1;
    let mut characters = prefix.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            lines += 1;
        } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
            lines += 1;
        }
    }
    lines
}

#[cfg(test)]
fn line_start(source: &str, offset: usize) -> usize {
    source[..offset]
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}')
                .then_some(index + character.len_utf8())
        })
        .unwrap_or(0)
}

#[cfg(test)]
fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
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
    reason = "serde_json::json keeps the focused JSX edge matrix concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-props-no-multi-spaces-v5.10.0.json"
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
        output: Option<String>,
        #[serde(default)]
        diagnostics: Vec<ExpectedDiagnostic>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        data: BTreeMap<String, String>,
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
        range: [u32; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedFix {
        range: [u32; 2],
        replacement_text: String,
    }

    fn run(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_props_no_multi_spaces(source, filename, options, &mut diagnostics);
        diagnostics
    }

    fn first_pass(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                usize::try_from(fix.range.start).expect("fix start fits usize")
                    ..usize::try_from(fix.range.end).expect("fix end fits usize"),
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    fn converge(source: &str, filename: Option<&str>, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, filename, options);
            let Some(next) = first_pass(&output, &diagnostics) else {
                return changed.then_some(output);
            };
            assert_ne!(next, output, "fix must make progress");
            output = next;
            changed = true;
        }
        panic!("fixes failed to converge");
    }

    fn location_at(source: &str, byte_offset: u32) -> (usize, usize) {
        let offset = usize::try_from(byte_offset).expect("offset fits usize");
        (
            line_at(source, offset),
            utf16_len(&source[line_start(source, offset)..offset]) + 1,
        )
    }

    #[test]
    fn replays_every_authored_pinned_case_with_exact_reports_ranges_and_fixes() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.generated.inventory.valid, 16);
        assert_eq!(fixture.generated.inventory.invalid, 12);
        assert_eq!(fixture.generated.inventory.diagnostics, 17);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 7);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 5);
        assert_eq!(fixture.generated.inventory.total, 28);

        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, Some("fixture.tsx"), &test_case.options).is_empty(),
                "authored valid case {index} reported diagnostics:\n{}",
                test_case.code
            );
        }

        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, Some("fixture.tsx"), &test_case.options);
            assert_eq!(
                diagnostics.len(),
                test_case.diagnostics.len(),
                "diagnostic count differs for authored invalid case {index}"
            );
            for (actual, expected) in diagnostics.iter().zip(&test_case.diagnostics) {
                assert_eq!(actual.message_id, expected.message_id, "case {index}");
                assert_eq!(actual.message, expected.message, "case {index}");
                assert_eq!(actual.data, expected.data, "case {index}");
                assert_eq!(
                    actual.range,
                    TextRange::new(expected.range[0], expected.range[1]),
                    "case {index}"
                );
                assert_eq!(
                    location_at(&test_case.code, actual.range.start),
                    (expected.line, expected.column),
                    "case {index} start"
                );
                assert_eq!(
                    location_at(&test_case.code, actual.range.end),
                    (expected.end_line, expected.end_column),
                    "case {index} end"
                );
                match (
                    actual
                        .suggestions
                        .first()
                        .and_then(|suggestion| suggestion.fixes.first()),
                    &expected.fix,
                ) {
                    (Some(actual), Some(expected)) => {
                        assert_eq!(
                            actual.range,
                            TextRange::new(expected.range[0], expected.range[1]),
                            "case {index} fix range"
                        );
                        assert_eq!(
                            actual.replacement_text, expected.replacement_text,
                            "case {index} replacement"
                        );
                    }
                    (None, None) => {}
                    _ => panic!("case {index} fixability differs"),
                }
            }
            assert_eq!(
                first_pass(&test_case.code, &diagnostics),
                test_case.output,
                "case {index} first pass"
            );
            assert_eq!(
                converge(&test_case.code, Some("fixture.tsx"), &test_case.options),
                test_case.output,
                "case {index} recursive output"
            );
        }
    }

    #[test]
    fn handles_generic_member_namespaced_spread_and_nested_openings_in_source_order() {
        let source = concat!(
            "<Outer<T>  child={<Inner.Member  {...props.value}   foo:bar />}",
            "   tail />"
        );
        let diagnostics = run(source, Some("fixture.tsx"), &Value::Null);
        assert_eq!(diagnostics.len(), 4);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| (
                    diagnostic.data["prop1"].as_str(),
                    diagnostic.data["prop2"].as_str()
                ))
                .collect::<Vec<_>>(),
            [
                ("Outer", "child"),
                ("child", "tail"),
                ("Inner.Member", "props.value"),
                ("props.value", "foo:bar"),
            ]
        );
    }

    #[test]
    fn preserves_exact_utf8_ranges_and_maps_astral_values_independently() {
        let source = "const 日本語 = <App<T>  foo=\"🦀\"   {...props} />;";
        let diagnostics = run(source, Some("fixture.tsx"), &Value::Null);
        assert_eq!(diagnostics.len(), 2);
        let foo_start = source.find("foo=").expect("foo");
        let spread_start = source.find("{...props}").expect("spread");
        assert_eq!(
            diagnostics[0].range,
            byte_range(foo_start, foo_start + "foo=\"🦀\"".len())
        );
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0].range,
            byte_range(source.find(">  foo").expect("generic gap") + 1, foo_start)
        );
        assert_eq!(
            diagnostics[1].range,
            byte_range(spread_start, spread_start + "{...props}".len())
        );
        assert_eq!(
            first_pass(source, &diagnostics).as_deref(),
            Some("const 日本語 = <App<T> foo=\"🦀\" {...props} />;")
        );
    }

    #[test]
    fn recognizes_every_ecmascript_line_terminator_and_comment_boundary() {
        for separator in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let source = format!("<App foo{separator}{separator}bar />");
            let diagnostics = run(&source, Some("fixture.tsx"), &Value::Null);
            assert_eq!(diagnostics.len(), 1, "separator {separator:?}");
            assert_eq!(diagnostics[0].message_id, NO_LINE_GAP_ID);
            assert!(diagnostics[0].suggestions.is_empty());
        }

        let source = "<App foo\n// first\n// second\n\nbar />";
        let diagnostics = run(source, Some("fixture.tsx"), &Value::Null);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message_id, NO_LINE_GAP_ID);
    }

    #[test]
    fn matches_upstream_wholesale_inline_gap_fix_and_ignores_fragments_and_malformed_code() {
        let source = "<><App foo /* keep? */  bar /></>";
        let diagnostics = run(source, Some("fixture.jsx"), &json!([{"ignored": true}]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            first_pass(source, &diagnostics).as_deref(),
            Some("<><App foo bar /></>")
        );

        assert!(run("<></>", Some("fixture.jsx"), &Value::Null).is_empty());
        assert!(run("<App  ", Some("fixture.tsx"), &Value::Null).is_empty());
        assert!(
            run(
                "const text = '<App  foo />'; // <App  bar />",
                Some("fixture.ts"),
                &Value::Null
            )
            .is_empty()
        );
    }
}
