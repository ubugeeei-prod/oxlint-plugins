//! Native AST implementation of stable
//! `@stylistic/jsx-self-closing-comp` v5.10.0.
//!
//! The upstream rule distinguishes React-compatible DOM names from
//! components by the first ASCII character of the rendered JSX name. It also
//! treats only truly empty elements or one LF-containing whitespace JSX text
//! child as empty; comments, fragments, NBSP, and same-line spaces stay intact.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXChild, JSXElement, JSXElementName};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-self-closing-comp";
const MESSAGE_ID: &str = "notSelfClosing";
const MESSAGE: &str = "Empty components are self-closing";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Configuration {
    component: bool,
    html: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            component: true,
            html: true,
        }
    }
}

/// Disallows explicit closing tags for configured empty JSX elements.
pub(crate) fn check_jsx_self_closing_comp(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let configuration = normalize_options(options);

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok())
        && parse_and_check(source, source_type, configuration, diagnostics)
    {
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, configuration, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    configuration: Configuration,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = SelfClosingVisitor {
        source,
        configuration,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct SelfClosingVisitor<'source, 'diagnostics> {
    source: &'source str,
    configuration: Configuration,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for SelfClosingVisitor<'_, '_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'ast>) {
        self.check(element);
        walk::walk_jsx_element(self, element);
    }
}

impl SelfClosingVisitor<'_, '_> {
    fn check(&mut self, element: &JSXElement<'_>) {
        let Some(closing_element) = &element.closing_element else {
            return;
        };
        let opening = &element.opening_element;
        let is_dom = is_dom_component(self.source, &opening.name);
        let configured =
            (self.configuration.component && is_component_shape(&opening.name) && !is_dom)
                || (self.configuration.html && is_dom);
        if !configured || !children_are_empty(&element.children) {
            return;
        }

        let opening_span = opening.span;
        let closing_span = closing_element.span;
        let fix_start = opening_span.end.saturating_sub(1);
        if opening_span.start > opening_span.end
            || fix_start >= closing_span.end
            || usize::try_from(closing_span.end).map_or(true, |end| end > self.source.len())
            || self
                .source
                .as_bytes()
                .get(usize::try_from(fix_start).unwrap_or(usize::MAX))
                != Some(&b'>')
        {
            return;
        }

        let fix = LintFix::replace_range(TextRange::new(fix_start, closing_span.end), " />");
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: MESSAGE.to_owned(),
            data: BTreeMap::new(),
            range: TextRange::new(opening_span.start, opening_span.end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message: MESSAGE.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }
}

fn normalize_options(options: &Value) -> Configuration {
    let first = match options {
        Value::Array(values) => values.first(),
        Value::Null => None,
        value => Some(value),
    };
    let Some(first) = first else {
        return Configuration::default();
    };
    if first.is_null() {
        return Configuration::default();
    }
    let Some(object) = first.as_object() else {
        return Configuration {
            component: false,
            html: false,
        };
    };

    Configuration {
        component: object.get("component").is_none_or(json_truthy),
        html: object.get("html").is_none_or(json_truthy),
    }
}

fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn is_component_shape(name: &JSXElementName<'_>) -> bool {
    matches!(
        name,
        JSXElementName::Identifier(_)
            | JSXElementName::IdentifierReference(_)
            | JSXElementName::MemberExpression(_)
    )
}

fn is_dom_component(source: &str, name: &JSXElementName<'_>) -> bool {
    source
        .as_bytes()
        .get(usize::try_from(name.span().start).unwrap_or(usize::MAX))
        .is_some_and(u8::is_ascii_lowercase)
}

fn children_are_empty(children: &[JSXChild<'_>]) -> bool {
    if children.is_empty() {
        return true;
    }
    let [JSXChild::Text(text)] = children else {
        return false;
    };
    multiline_spaces(text.value.as_str())
}

fn multiline_spaces(value: &str) -> bool {
    value.contains('\n')
        && value
            .chars()
            .all(|character| character != '\u{00a0}' && is_ecmascript_whitespace(character))
}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\u{000b}' | '\u{000c}' | '\r' | '\n' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "Compatibility matrices use serde_json and assertion macros only in tests."
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-self-closing-comp-v5.10.0.json"
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
        logical_valid: usize,
        logical_invalid: usize,
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        fixable_invalid: usize,
        total: usize,
    }

    #[derive(Deserialize)]
    struct TestCase {
        code: String,
        #[serde(default)]
        options: Value,
        #[serde(default)]
        diagnostics: Vec<ExpectedDiagnostic>,
        #[serde(default)]
        output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        data: BTreeMap<String, String>,
        range: [u32; 2],
        location: ExpectedLocation,
        fix: ExpectedFix,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedLocation {
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedFix {
        range: [u32; 2],
        replacement_text: String,
    }

    fn run(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_self_closing_comp(source, filename, options, &mut diagnostics);
        diagnostics
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
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
        let column = source[line_start(source, offset)..offset]
            .encode_utf16()
            .count()
            + 1;
        (line, column)
    }

    #[test]
    fn replays_every_pinned_authored_case_for_jsx_and_tsx_exactly() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.generated.version, "v5.10.0");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.generated.inventory.logical_valid, 35);
        assert_eq!(fixture.generated.inventory.logical_invalid, 12);
        assert_eq!(fixture.generated.inventory.valid, 105);
        assert_eq!(fixture.generated.inventory.invalid, 36);
        assert_eq!(fixture.generated.inventory.diagnostics, 36);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 36);
        assert_eq!(fixture.generated.inventory.total, 141);

        for filename in ["fixture.jsx", "fixture.tsx"] {
            for (index, test_case) in fixture.valid.iter().enumerate() {
                assert!(
                    run(&test_case.code, Some(filename), &test_case.options).is_empty(),
                    "{filename} upstream valid case {index} reported diagnostics:\n{}",
                    test_case.code
                );
            }

            for (index, test_case) in fixture.invalid.iter().enumerate() {
                let diagnostics = run(&test_case.code, Some(filename), &test_case.options);
                assert_eq!(
                    diagnostics.len(),
                    test_case.diagnostics.len(),
                    "{filename} invalid case {index}"
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
                        (expected.location.line, expected.location.column),
                        "case {index}"
                    );
                    assert_eq!(
                        location_at(&test_case.code, actual.range.end),
                        (expected.location.end_line, expected.location.end_column),
                        "case {index}"
                    );
                    let suggestion = actual.suggestions.first().expect("fix suggestion");
                    assert_eq!(suggestion.message_id, MESSAGE_ID, "case {index}");
                    assert_eq!(suggestion.message, MESSAGE, "case {index}");
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
                    apply_fixes(&test_case.code, &diagnostics),
                    test_case.output,
                    "{filename} first-pass output differs for case {index}"
                );
            }
        }
    }

    #[test]
    fn honors_component_html_defaults_partial_overrides_and_disabled_modes() {
        let source = "<Widget></Widget>;<div></div>;";
        assert_eq!(run(source, Some("fixture.tsx"), &json!([])).len(), 2);
        assert_eq!(
            run(
                source,
                Some("fixture.tsx"),
                &json!([{ "component": false }])
            )
            .len(),
            1
        );
        assert_eq!(
            run(source, Some("fixture.tsx"), &json!([{ "html": false }])).len(),
            1
        );
        assert!(
            run(
                source,
                Some("fixture.tsx"),
                &json!([{ "component": false, "html": false }])
            )
            .is_empty()
        );
    }

    #[test]
    fn matches_lf_whitespace_nbsp_and_all_line_terminator_boundaries() {
        for source in [
            "<Widget>\n</Widget>",
            "<Widget>\r\n</Widget>",
            "<Widget>\t\u{1680}\u{2007}\u{202f}\n\u{3000}</Widget>",
        ] {
            assert_eq!(
                run(source, Some("fixture.tsx"), &Value::Null).len(),
                1,
                "{source:?}"
            );
        }

        for source in [
            "<Widget> </Widget>",
            "<Widget>\r</Widget>",
            "<Widget>\u{2028}</Widget>",
            "<Widget>\u{2029}</Widget>",
            "<Widget>\n\u{00a0}</Widget>",
            "<Widget>&nbsp;</Widget>",
        ] {
            assert!(
                run(source, Some("fixture.tsx"), &Value::Null).is_empty(),
                "{source:?}"
            );
        }
    }

    #[test]
    fn preserves_fragments_comments_namespaces_members_and_nested_source_order() {
        let source = concat!(
            "<><Widget></Widget><div></div></>;",
            "<Widget>{/* keep */}</Widget>;",
            "<foo:bar></foo:bar>;",
            "<Foo:bar></Foo:bar>;",
            "<foo.Part></foo.Part>;",
            "<Foo.Part></Foo.Part>;",
            "<this.Part></this.Part>;",
        );
        for filename in ["fixture.jsx", "fixture.tsx"] {
            let diagnostics = run(source, Some(filename), &Value::Null);
            assert_eq!(diagnostics.len(), 6, "{filename}");
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.range.start)
                    .collect::<Vec<_>>(),
                vec![2, 19, 64, 104, 126, 148],
                "{filename}"
            );
        }
    }

    #[test]
    fn supports_tsx_generics_unicode_byte_ranges_and_exact_fixes() {
        let source = "const 日本語 = <Widget<string> data=\"値\"></Widget>;";
        let diagnostics = run(source, Some("fixture.tsx"), &Value::Null);
        assert_eq!(diagnostics.len(), 1);
        let opening_end =
            u32::try_from(source.find("></Widget>").unwrap() + 1).expect("opening end fits u32");
        let expected_start =
            u32::try_from("const 日本語 = ".len()).expect("UTF-8 prefix length fits u32");
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(expected_start, opening_end)
        );
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0].range,
            TextRange::new(
                opening_end - 1,
                u32::try_from(source.rfind('>').unwrap() + 1).unwrap()
            )
        );
        assert_eq!(
            apply_fixes(source, &diagnostics).as_deref(),
            Some("const 日本語 = <Widget<string> data=\"値\" />;")
        );
    }

    #[test]
    fn keeps_nonempty_children_and_invalid_syntax_silent_and_options_total() {
        for source in [
            "<Widget>text</Widget>",
            "<Widget>{value}</Widget>",
            "<Widget><Child /></Widget>",
            "<Widget>{}</Widget>",
            "<Widget>",
        ] {
            assert!(
                run(source, Some("fixture.tsx"), &Value::Null).is_empty(),
                "{source:?}"
            );
        }

        for options in [
            Value::Null,
            json!([]),
            json!([null]),
            json!([{ "component": "bad", "html": 1 }]),
            json!({ "component": true }),
        ] {
            assert_eq!(
                run("<Widget></Widget>", Some("fixture.tsx"), &options).len(),
                1,
                "{options}"
            );
        }
        for options in [
            json!(["bad"]),
            json!([0]),
            json!([{ "component": null, "html": false }]),
        ] {
            assert!(
                run("<Widget></Widget>", Some("fixture.tsx"), &options).is_empty(),
                "{options}"
            );
        }
    }

    #[test]
    fn retains_upstream_spacing_even_when_opening_already_has_space() {
        let source = "<Widget ></Widget>";
        let diagnostics = run(source, Some("fixture.tsx"), &Value::Null);
        assert_eq!(
            apply_fixes(source, &diagnostics).as_deref(),
            Some("<Widget  />")
        );
    }
}
