//! Native AST implementation of stable
//! `@stylistic/jsx-closing-tag-location`.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXClosingElement, JSXClosingFragment, JSXElement, JSXFragment};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-closing-tag-location";
const ON_OWN_LINE_ID: &str = "onOwnLine";
const ON_OWN_LINE_MESSAGE: &str =
    "Closing tag of a multiline JSX expression must be on its own line.";
const MATCH_INDENT_ID: &str = "matchIndent";
const MATCH_INDENT_MESSAGE: &str = "Expected closing tag to match indentation of opening.";
const ALIGN_WITH_OPENING_ID: &str = "alignWithOpening";
const ALIGN_WITH_OPENING_MESSAGE: &str =
    "Expected closing tag to be aligned with the line containing the opening tag";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Alignment {
    Tag,
    Line,
}

/// Enforces the closing-tag location for multiline JSX elements and fragments.
pub(crate) fn check_jsx_closing_tag_location(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let alignment = normalize_option(options);

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok())
        && parse_and_check(source, source_type, alignment, diagnostics)
    {
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, alignment, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    alignment: Alignment,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = ClosingTagVisitor {
        source,
        alignment,
        opening_stack: Vec::new(),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct ClosingTagVisitor<'source, 'diagnostics> {
    source: &'source str,
    alignment: Alignment,
    opening_stack: Vec<Span>,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for ClosingTagVisitor<'_, '_> {
    fn visit_jsx_element(&mut self, node: &JSXElement<'ast>) {
        self.opening_stack.push(node.opening_element.span);
        walk::walk_jsx_element(self, node);
        self.opening_stack.pop();
    }

    fn visit_jsx_closing_element(&mut self, node: &JSXClosingElement<'ast>) {
        if let Some(opening) = self.opening_stack.last().copied() {
            self.check(opening, node.span);
        }
        walk::walk_jsx_closing_element(self, node);
    }

    fn visit_jsx_fragment(&mut self, node: &JSXFragment<'ast>) {
        self.opening_stack.push(node.opening_fragment.span);
        walk::walk_jsx_fragment(self, node);
        self.opening_stack.pop();
    }

    fn visit_jsx_closing_fragment(&mut self, node: &JSXClosingFragment) {
        if let Some(opening) = self.opening_stack.last().copied() {
            self.check(opening, node.span);
        }
        walk::walk_jsx_closing_fragment(self, node);
    }
}

impl ClosingTagVisitor<'_, '_> {
    fn check(&mut self, opening: Span, closing: Span) {
        let (Ok(opening_start), Ok(closing_start), Ok(closing_end)) = (
            usize::try_from(opening.start),
            usize::try_from(closing.start),
            usize::try_from(closing.end),
        ) else {
            return;
        };
        if opening_start > self.source.len()
            || closing_start > self.source.len()
            || closing_end > self.source.len()
        {
            return;
        }

        let opening_line_start = line_start(self.source, opening_start);
        let closing_line_start = line_start(self.source, closing_start);
        if opening_line_start == closing_line_start {
            return;
        }

        let opening_column = utf16_len(&self.source[opening_line_start..opening_start]);
        let opening_indent = leading_whitespace_utf16(&self.source[opening_line_start..]);
        let closing_column = utf16_len(&self.source[closing_line_start..closing_start]);
        let expected_column = match self.alignment {
            Alignment::Tag => opening_column,
            Alignment::Line => opening_indent,
        };
        if closing_column == expected_column {
            return;
        }

        let first_in_line = self.source[closing_line_start..closing_start]
            .chars()
            .all(is_ecmascript_whitespace);
        let (message_id, message) = if first_in_line {
            match self.alignment {
                Alignment::Tag => (MATCH_INDENT_ID, MATCH_INDENT_MESSAGE),
                Alignment::Line => (ALIGN_WITH_OPENING_ID, ALIGN_WITH_OPENING_MESSAGE),
            }
        } else {
            (ON_OWN_LINE_ID, ON_OWN_LINE_MESSAGE)
        };

        let mut replacement = String::new();
        let fix_start = if first_in_line {
            closing_line_start
        } else {
            replacement.push('\n');
            closing_start
        };
        replacement.extend(std::iter::repeat_n(' ', expected_column));
        let fix = LintFix::replace_range(byte_range(fix_start, closing_start), replacement);
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range: byte_range(closing_start, closing_end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }
}

fn normalize_option(options: &Value) -> Alignment {
    let keyword = match options {
        Value::Array(values) => values.first().and_then(Value::as_str),
        value => value.as_str(),
    };
    if keyword == Some("line-aligned") {
        Alignment::Line
    } else {
        Alignment::Tag
    }
}

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

fn leading_whitespace_utf16(line: &str) -> usize {
    line.chars()
        .take_while(|character| is_ecmascript_whitespace(*character))
        .map(char::len_utf16)
        .sum()
}

fn is_ecmascript_whitespace(character: char) -> bool {
    character.is_whitespace() || character == '\u{feff}'
}

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
    reason = "serde_json::json keeps the focused JSX option matrix concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-closing-tag-location-v5.10.0.json"
    ));

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
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
        fix: ExpectedFix,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedFix {
        range: [u32; 2],
        replacement_text: String,
    }

    fn run(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_closing_tag_location(source, filename, options, &mut diagnostics);
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
        let mut line = 1;
        let mut characters = source[..offset].chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\r' {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                line += 1;
            } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
                line += 1;
            }
        }
        let column = utf16_len(&source[line_start(source, offset)..offset]) + 1;
        (line, column)
    }

    #[test]
    fn replays_every_pinned_upstream_case_with_exact_reports_and_fixes() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.valid.len(), 28);
        assert_eq!(fixture.invalid.len(), 16);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .flat_map(|test_case| &test_case.diagnostics)
                .count(),
            16
        );

        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, Some("fixture.tsx"), &test_case.options).is_empty(),
                "upstream valid case {index} reported diagnostics:\n{}",
                test_case.code
            );
        }

        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, Some("fixture.tsx"), &test_case.options);
            assert_eq!(
                diagnostics.len(),
                test_case.diagnostics.len(),
                "diagnostic count differs for upstream invalid case {index}"
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
                    "case {index}"
                );
                assert_eq!(
                    location_at(&test_case.code, actual.range.end),
                    (expected.end_line, expected.end_column),
                    "case {index}"
                );
                let suggestion = &actual.suggestions[0];
                assert_eq!(suggestion.message_id, expected.message_id, "case {index}");
                assert_eq!(suggestion.message, expected.message, "case {index}");
                assert_eq!(suggestion.fixes.len(), 1, "case {index}");
                let fix = &suggestion.fixes[0];
                assert_eq!(
                    fix.range,
                    TextRange::new(expected.fix.range[0], expected.fix.range[1]),
                    "case {index}"
                );
                assert_eq!(
                    fix.replacement_text, expected.fix.replacement_text,
                    "case {index}"
                );
            }
            assert_eq!(
                first_pass(&test_case.code, &diagnostics),
                test_case.output,
                "first-pass output differs for upstream invalid case {index}"
            );
            assert_eq!(
                converge(&test_case.code, Some("fixture.tsx"), &test_case.options),
                test_case.output,
                "converged output differs for upstream invalid case {index}"
            );
        }
    }

    #[test]
    fn covers_all_messages_options_elements_fragments_and_nested_order() {
        let source =
            "const view = <Outer>\n  <>\n    <Inner>\n      text</Inner>\n    </>\n    </Outer>;";
        let diagnostics = run(source, Some("fixture.tsx"), &json!([]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [ON_OWN_LINE_ID, MATCH_INDENT_ID, MATCH_INDENT_ID]
        );

        let line_aligned = "const view = <App>\n  content\n            </App>;";
        let diagnostics = run(line_aligned, Some("fixture.jsx"), &json!(["line-aligned"]));
        assert_eq!(diagnostics[0].message_id, ALIGN_WITH_OPENING_ID);
        assert_eq!(
            first_pass(line_aligned, &diagnostics).as_deref(),
            Some("const view = <App>\n  content\n</App>;")
        );

        assert!(
            run(
                "const view = <App>\n  content\n             </App>;",
                Some("fixture.jsx"),
                &json!([])
            )
            .is_empty()
        );
    }

    #[test]
    fn recognizes_every_ecmascript_line_terminator_and_preserves_upstream_lf_fix() {
        for linebreak in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("<App>{linebreak}content</App>");
            let diagnostics = run(&source, Some("fixture.jsx"), &json!([]));
            assert_eq!(diagnostics.len(), 1, "{linebreak:?}");
            assert_eq!(diagnostics[0].message_id, ON_OWN_LINE_ID);
            assert_eq!(
                diagnostics[0].suggestions[0].fixes[0].replacement_text, "\n",
                "{linebreak:?}"
            );
            assert_eq!(
                converge(&source, Some("fixture.jsx"), &json!([])).as_deref(),
                Some(format!("<App>{linebreak}content\n</App>").as_str()),
                "{linebreak:?}"
            );
        }
    }

    #[test]
    fn preserves_utf8_byte_ranges_while_matching_utf16_indentation() {
        let source = "const 日本語 = <App>\n  child</App>;";
        let diagnostics = run(source, Some("fixture.tsx"), &json!([]));
        let start = source.find("</App>").expect("closing tag");
        assert_eq!(
            diagnostics[0].range,
            byte_range(start, start + "</App>".len())
        );
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0].replacement_text,
            format!("\n{}", " ".repeat("const 日本語 = ".encode_utf16().count()))
        );
    }

    #[test]
    fn handles_tsx_generics_member_names_namespaces_and_js_fallback() {
        for (source, filename) in [
            (
                "const view: JSX.Element = <UI.Panel<T>>\n  x</UI.Panel>;",
                "fixture.tsx",
            ),
            ("const view = <svg:path>\n  x</svg:path>;", "fixture.jsx"),
            ("const view = <App>\n  x</App>;", "fixture.js"),
        ] {
            assert_eq!(run(source, Some(filename), &json!([])).len(), 1, "{source}");
        }
    }

    #[test]
    fn ignores_single_line_self_closing_non_jsx_and_parse_failures() {
        for (source, filename) in [
            ("const view = <App>content</App>;", "fixture.tsx"),
            ("const view = <App />;", "fixture.tsx"),
            ("const less = left < right;", "fixture.ts"),
            ("const view = <App>", "fixture.tsx"),
        ] {
            assert!(
                run(source, Some(filename), &json!([])).is_empty(),
                "{source}"
            );
        }
    }
}
