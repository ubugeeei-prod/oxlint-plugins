//! Native implementation of `@stylistic/jsx-child-element-spacing`.
//!
//! The rule examines only direct JSX children. When text beginning or ending
//! on a different source line touches a lowercase inline HTML element, JSX's
//! whitespace normalization can make the rendered spacing ambiguous. The
//! upstream rule deliberately reports this ambiguity without offering a fix.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXChild, JSXElement, JSXElementName, JSXFragment};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::Value;

use crate::{LintDiagnostic, TextRange};

const RULE_NAME: &str = "jsx-child-element-spacing";
const SPACING_AFTER_PREV: &str = "Ambiguous spacing after previous element";
const SPACING_BEFORE_NEXT: &str = "Ambiguous spacing before next element";

const INLINE_ELEMENTS: &[&str] = &[
    "a", "abbr", "acronym", "b", "bdo", "big", "button", "cite", "code", "dfn", "em", "i", "img",
    "input", "kbd", "label", "map", "object", "q", "samp", "script", "select", "small", "span",
    "strong", "sub", "sup", "textarea", "tt", "var",
];

pub(crate) fn check_jsx_child_element_spacing(
    source: &str,
    filename: Option<&str>,
    _options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, diagnostics);
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

    let mut checker = JsxChildElementSpacing { diagnostics };
    checker.visit_program(&parsed.program);
    true
}

struct JsxChildElementSpacing<'diagnostics> {
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxChildElementSpacing<'_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'ast>) {
        self.check_children(&element.children);
        walk::walk_jsx_element(self, element);
    }

    fn visit_jsx_fragment(&mut self, fragment: &JSXFragment<'ast>) {
        self.check_children(&fragment.children);
        walk::walk_jsx_fragment(self, fragment);
    }
}

impl JsxChildElementSpacing<'_> {
    fn check_children(&mut self, children: &[JSXChild<'_>]) {
        for (index, child) in children.iter().enumerate() {
            let JSXChild::Text(text) = child else {
                continue;
            };
            let previous = index
                .checked_sub(1)
                .and_then(|previous| children.get(previous));
            let next = children.get(index + 1);

            if previous.is_none() && next.is_none() {
                continue;
            }
            if previous.is_some_and(|candidate| inline_element(candidate).is_none())
                || next.is_some_and(|candidate| inline_element(candidate).is_none())
            {
                continue;
            }

            if let Some((element, name)) = previous.and_then(inline_element)
                && text_follows_element(text.value.as_str())
            {
                self.report(
                    "spacingAfterPrev",
                    SPACING_AFTER_PREV,
                    name,
                    element.span.end,
                );
            } else if let Some((element, name)) = next.and_then(inline_element)
                && text_precedes_element(text.value.as_str())
            {
                self.report(
                    "spacingBeforeNext",
                    SPACING_BEFORE_NEXT,
                    name,
                    element.span.start,
                );
            }
        }
    }

    fn report(&mut self, message_id: &str, message: &str, element: &str, offset: u32) {
        let mut rendered_message = String::with_capacity(message.len() + 1 + element.len());
        rendered_message.push_str(message);
        rendered_message.push(' ');
        rendered_message.push_str(element);
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE_NAME.to_owned(),
            message_id: message_id.to_owned(),
            message: rendered_message,
            data: BTreeMap::from([("element".to_owned(), element.to_owned())]),
            range: TextRange::new(offset, offset),
            suggestions: Vec::new(),
        });
    }
}

fn inline_element<'node, 'ast>(
    child: &'node JSXChild<'ast>,
) -> Option<(&'node JSXElement<'ast>, &'node str)> {
    let JSXChild::Element(element) = child else {
        return None;
    };
    let JSXElementName::Identifier(identifier) = &element.opening_element.name else {
        return None;
    };
    let name = identifier.name.as_str();
    INLINE_ELEMENTS.contains(&name).then_some((element, name))
}

/// Equivalent to upstream's
/// `/^[horizontal-whitespace]*\n\s*\S/`.
fn text_follows_element(text: &str) -> bool {
    let Some(newline) = text.find('\n') else {
        return false;
    };
    text[..newline].chars().all(is_horizontal_whitespace)
        && text[newline + 1..]
            .chars()
            .any(|character| !is_ecmascript_whitespace(character))
}

/// Equivalent to upstream's
/// `/\S[horizontal-whitespace]*\n\s*$/`.
fn text_precedes_element(text: &str) -> bool {
    for (newline, _) in text.match_indices('\n') {
        if !text[newline + 1..].chars().all(is_ecmascript_whitespace) {
            continue;
        }

        let before = &text[..newline];
        let significant = before.trim_end_matches(is_horizontal_whitespace);
        if significant
            .chars()
            .next_back()
            .is_some_and(|character| !is_ecmascript_whitespace(character))
        {
            return true;
        }
    }
    false
}

fn is_horizontal_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\u{000b}' | '\u{000c}' | '\r' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn is_ecmascript_whitespace(character: char) -> bool {
    character == '\n' || is_horizontal_whitespace(character)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "Dense compatibility matrices use format and serde_json macros only in tests."
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str) -> Vec<LintDiagnostic> {
        run_with(source, Some("fixture.tsx"), &Value::Null)
    }

    fn run_with(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_child_element_spacing(source, filename, options, &mut diagnostics);
        diagnostics
    }

    fn ids(diagnostics: &[LintDiagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id.as_str())
            .collect()
    }

    fn upstream_fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-child-element-spacing-v5.10.0.json"
        ))
        .expect("generated jsx-child-element-spacing fixture is valid JSON")
    }

    #[test]
    fn accepts_all_62_expanded_stable_v5_10_0_valid_fixtures() {
        let fixture = upstream_fixture();
        let generated = &fixture["__generated"];
        assert_eq!(generated["version"], "v5.10.0");
        assert_eq!(
            generated["commit"],
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(generated["inventory"]["logicalValid"], 21);
        assert_eq!(generated["inventory"]["valid"], 62);
        assert_eq!(generated["inventory"]["invalid"], 20);
        assert_eq!(generated["inventory"]["diagnostics"], 23);

        let valid = fixture["valid"].as_array().expect("valid fixture array");
        assert_eq!(valid.len(), 62);
        for (index, test) in valid.iter().enumerate() {
            let source = test["code"].as_str().expect("valid fixture code");
            assert!(
                run(source).is_empty(),
                "upstream valid fixture {index} reported diagnostics:\n{source}"
            );
        }
    }

    #[test]
    fn replays_all_20_expanded_invalid_fixtures_and_23_diagnostics_exactly() {
        let fixture = upstream_fixture();
        let invalid = fixture["invalid"]
            .as_array()
            .expect("invalid fixture array");
        assert_eq!(invalid.len(), 20);
        assert_eq!(
            invalid
                .iter()
                .map(|test| test["expectedDiagnostics"].as_array().unwrap().len())
                .sum::<usize>(),
            23
        );

        for (index, test) in invalid.iter().enumerate() {
            let source = test["code"].as_str().expect("invalid fixture code");
            let expected = test["expectedDiagnostics"]
                .as_array()
                .expect("expected diagnostics");
            let diagnostics = run(source);
            assert_eq!(
                diagnostics.len(),
                expected.len(),
                "diagnostic count differs for upstream invalid fixture {index}\n{source}"
            );

            for (diagnostic, expected) in diagnostics.iter().zip(expected) {
                assert_eq!(
                    diagnostic.message_id,
                    expected["messageId"].as_str().unwrap()
                );
                assert_eq!(diagnostic.message, expected["message"].as_str().unwrap());
                assert_eq!(
                    diagnostic.data["element"],
                    expected["data"]["element"].as_str().unwrap()
                );
                assert_eq!(
                    u64::from(diagnostic.range.start),
                    expected["range"]["start"].as_u64().unwrap()
                );
                assert_eq!(
                    u64::from(diagnostic.range.end),
                    expected["range"]["end"].as_u64().unwrap()
                );
                assert!(diagnostic.range.is_empty());
                assert!(diagnostic.suggestions.is_empty());
                assert!(expected["fix"].is_null());
            }
            assert!(test["output"].is_null());
        }
    }

    #[test]
    fn reports_both_ambiguous_directions_with_exact_zero_width_ranges() {
        let before = "<App>word\n<a>link</a></App>";
        let before_diagnostics = run(before);
        let before_offset = u32::try_from(before.find("<a>").unwrap()).unwrap();
        assert_eq!(ids(&before_diagnostics), ["spacingBeforeNext"]);
        assert_eq!(
            before_diagnostics[0].range,
            TextRange::new(before_offset, before_offset)
        );
        assert_eq!(
            before_diagnostics[0].message,
            "Ambiguous spacing before next element a"
        );
        assert_eq!(
            before_diagnostics[0].data,
            BTreeMap::from([("element".to_owned(), "a".to_owned())])
        );
        assert!(before_diagnostics[0].suggestions.is_empty());

        let after = "<App><a>link</a>\nword</App>";
        let after_diagnostics = run(after);
        let after_offset = u32::try_from(after.find("</a>").unwrap() + "</a>".len()).unwrap();
        assert_eq!(ids(&after_diagnostics), ["spacingAfterPrev"]);
        assert_eq!(
            after_diagnostics[0].range,
            TextRange::new(after_offset, after_offset)
        );
        assert_eq!(
            after_diagnostics[0].message,
            "Ambiguous spacing after previous element a"
        );
        assert!(after_diagnostics[0].suggestions.is_empty());
    }

    #[test]
    fn previous_element_report_wins_when_both_patterns_match() {
        let source = "<App><a />\nword\n<b /></App>";
        let diagnostics = run(source);
        let previous_end = u32::try_from(source.find("<a />").unwrap() + "<a />".len()).unwrap();
        assert_eq!(ids(&diagnostics), ["spacingAfterPrev"]);
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(previous_end, previous_end)
        );
        assert_eq!(diagnostics[0].data["element"], "a");
    }

    #[test]
    fn recognizes_every_upstream_inline_html_element_in_both_directions() {
        for element in INLINE_ELEMENTS {
            let before = format!("<App>word\n<{element} /></App>");
            let after = format!("<App><{element} />\nword</App>");

            let before_diagnostics = run(&before);
            assert_eq!(
                ids(&before_diagnostics),
                ["spacingBeforeNext"],
                "before <{element}>"
            );
            assert_eq!(before_diagnostics[0].data["element"], *element);

            let after_diagnostics = run(&after);
            assert_eq!(
                ids(&after_diagnostics),
                ["spacingAfterPrev"],
                "after <{element}>"
            );
            assert_eq!(after_diagnostics[0].data["element"], *element);
        }
    }

    #[test]
    fn excludes_br_block_component_member_namespaced_and_fragment_neighbors() {
        let cases = [
            "<App>word\n<br /></App>",
            "<App>word\n<p /></App>",
            "<App>word\n<A /></App>",
            "<App>word\n<UI.a /></App>",
            "<App>word\n<svg:a /></App>",
            "<App>word\n<></></App>",
            "<App><br />\nword</App>",
            "<App><p />\nword</App>",
            "<App><A />\nword</App>",
            "<App><UI.a />\nword</App>",
            "<App><svg:a />\nword</App>",
            "<App><></>\nword</App>",
        ];

        for source in cases {
            assert!(run(source).is_empty(), "false positive for {source}");
        }
    }

    #[test]
    fn expression_comments_and_explicit_space_expressions_break_text_adjacency() {
        let cases = [
            "<App>word\n{/* comment */}<a /></App>",
            "<App><a />{/* comment */}\nword</App>",
            "<App>word\n{' '}<a /></App>",
            "<App><a />{' '}\nword</App>",
            "<App>word\n{value}<a /></App>",
            "<App><a />{value}\nword</App>",
        ];

        for source in cases {
            assert!(run(source).is_empty(), "false positive for {source}");
        }
    }

    #[test]
    fn matches_the_exact_ecmascript_whitespace_sets_and_requires_literal_lf() {
        let horizontal = [
            '\t', '\u{000b}', '\u{000c}', '\r', ' ', '\u{00a0}', '\u{1680}', '\u{2000}',
            '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
        ];

        for whitespace in horizontal {
            let after = format!("<App><a />{whitespace}\n{whitespace}word</App>");
            let before = format!("<App>word{whitespace}\n{whitespace}<a /></App>");
            assert_eq!(
                ids(&run(&after)),
                ["spacingAfterPrev"],
                "following U+{:04X}",
                whitespace as u32
            );
            assert_eq!(
                ids(&run(&before)),
                ["spacingBeforeNext"],
                "preceding U+{:04X}",
                whitespace as u32
            );
        }

        assert!(run("<App><a />\u{2028}word</App>").is_empty());
        assert!(run("<App>word\u{2029}<a /></App>").is_empty());

        // U+0085 is Unicode whitespace, but is intentionally not in
        // ECMAScript's `\\s` set.
        assert_eq!(
            ids(&run("<App><a />\n\u{0085}word</App>")),
            ["spacingAfterPrev"]
        );
        assert!(run("<App>word\n\u{0085}<a /></App>").is_empty());
    }

    #[test]
    fn supports_crlf_and_reports_utf8_byte_offsets() {
        let source = "<App>日本語\r\n<a>リンク</a>\r\n後続</App>";
        let diagnostics = run(source);
        let element_start = u32::try_from(source[..source.find("<a>").unwrap()].len()).unwrap();
        let element_end =
            u32::try_from(source[..source.find("</a>").unwrap() + "</a>".len()].len()).unwrap();

        assert_eq!(ids(&diagnostics), ["spacingBeforeNext", "spacingAfterPrev"]);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [
                TextRange::new(element_start, element_start),
                TextRange::new(element_end, element_end),
            ]
        );
    }

    #[test]
    fn traverses_parent_before_nested_children_like_upstream_listeners() {
        let source = "<App>outer\n<a><span>x</span>\ninner</a></App>";
        let diagnostics = run(source);
        assert_eq!(ids(&diagnostics), ["spacingBeforeNext", "spacingAfterPrev"]);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data["element"].as_str())
                .collect::<Vec<_>>(),
            ["a", "span"]
        );
    }

    #[test]
    fn handles_fragments_tsx_types_and_no_filename_fallback() {
        let fragment = "<>word\n<a /></>";
        assert_eq!(ids(&run(fragment)), ["spacingBeforeNext"]);

        let typescript =
            "const view: JSX.Element = <App>word\n<a data-value={value as string} /></App>;";
        assert_eq!(ids(&run(typescript)), ["spacingBeforeNext"]);
        assert_eq!(
            ids(&run_with(typescript, None, &json!([{"ignored": true}]))),
            ["spacingBeforeNext"]
        );
    }

    #[test]
    fn ignores_options_and_invalid_or_non_jsx_syntax() {
        let source = "<App>word\n<a /></App>";
        assert_eq!(
            run_with(
                source,
                Some("fixture.tsx"),
                &json!(["unused", {"anything": true}])
            ),
            run(source)
        );
        assert!(run("<App>word\n<a></App>").is_empty());
        assert!(run_with(source, Some("fixture.ts"), &Value::Null).is_empty());
    }

    #[test]
    fn accepts_nonambiguous_boundary_and_text_cases() {
        let cases = [
            "<App>only text</App>",
            "<App><a /></App>",
            "<App>word<a />word</App>",
            "<App> \n<a /></App>",
            "<App><a />\n </App>",
            "<App>\n<a /></App>",
            "<App><a />\n</App>",
            "<App>{/* before */}<a />{/* after */}</App>",
        ];

        for source in cases {
            assert!(run(source).is_empty(), "false positive for {source}");
        }
    }
}
