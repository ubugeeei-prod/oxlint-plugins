//! Native implementation of stable `@stylistic/jsx-max-props-per-line`.
//!
//! Oxc supplies exact JSX opening-element and attribute boundaries. The rule
//! partitions attributes by the physical line of adjacent attribute tokens,
//! reports the first excess prop on each overfull line, and reproduces the
//! upstream whole-line repartitioning fix.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXAttributeItem, JSXAttributeName, JSXOpeningElement};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;

const RULE: &str = "jsx-max-props-per-line";
const MESSAGE_ID: &str = "newLine";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Options {
    single: Option<usize>,
    multi: Option<usize>,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let Some(configuration) = first_option(options).and_then(Value::as_object) else {
            return Self {
                single: Some(1),
                multi: Some(1),
            };
        };
        let Some(maximum) = configuration.get("maximum") else {
            return Self {
                single: if configuration.get("when").and_then(Value::as_str) == Some("multiline") {
                    None
                } else {
                    Some(1)
                },
                multi: Some(1),
            };
        };

        if let Some(maximum) = positive_integer(maximum) {
            return Self {
                single: if configuration.get("when").and_then(Value::as_str) == Some("multiline") {
                    None
                } else {
                    Some(maximum)
                },
                multi: Some(maximum),
            };
        }

        if let Some(maximum) = maximum.as_object() {
            return Self {
                single: maximum.get("single").and_then(positive_integer),
                multi: maximum.get("multi").and_then(positive_integer),
            };
        }

        Self {
            single: Some(1),
            multi: Some(1),
        }
    }
}

fn positive_integer(value: &Value) -> Option<usize> {
    value
        .as_u64()
        .filter(|value| *value > 0)
        .and_then(|value| usize::try_from(value).ok())
}

pub(crate) fn check_jsx_max_props_per_line(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_json(options);
    let first_diagnostic = diagnostics.len();

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

    diagnostics[first_diagnostic..]
        .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
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

    let mut visitor = MaxPropsVisitor {
        source,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct MaxPropsVisitor<'source, 'diagnostics> {
    source: &'source str,
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for MaxPropsVisitor<'_, '_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        self.check(element);
        walk::walk_jsx_opening_element(self, element);
    }
}

impl MaxPropsVisitor<'_, '_> {
    fn check(&mut self, element: &JSXOpeningElement<'_>) {
        let Some(first_attribute) = element.attributes.first() else {
            return;
        };
        let single_line = same_line(self.source, element.span.start, element.span.end);
        let applicable = if single_line {
            self.options.single
        } else {
            self.options.multi
        };
        let Some(maximum) = applicable else {
            return;
        };

        let mut line_start = 0;
        let mut previous = first_attribute.span();
        for index in 1..=element.attributes.len() {
            let continues_line = element.attributes.get(index).is_some_and(|attribute| {
                same_line(self.source, previous.end, attribute.span().start)
            });
            if continues_line {
                previous = element.attributes[index].span();
                continue;
            }

            let line = &element.attributes[line_start..index];
            if line.len() > maximum {
                self.report(line, maximum);
            }
            line_start = index;
            if let Some(attribute) = element.attributes.get(index) {
                previous = attribute.span();
            }
        }
    }

    fn report(&mut self, line: &[JSXAttributeItem<'_>], maximum: usize) {
        let violating = &line[maximum];
        let prop = prop_name(self.source, violating);
        let mut message = String::from("Prop `");
        message.push_str(&prop);
        message.push_str("` must be placed on a new line");
        let data = BTreeMap::from([("prop".to_owned(), prop)]);
        let Some(fix) = line_fix(self.source, line, maximum) else {
            return;
        };
        let span = violating.span();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: message.clone(),
            data,
            range: TextRange::new(span.start, span.end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message,
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }
}

fn prop_name(source: &str, attribute: &JSXAttributeItem<'_>) -> String {
    match attribute {
        JSXAttributeItem::Attribute(attribute) => match &attribute.name {
            JSXAttributeName::Identifier(identifier) => identifier.name.as_str().to_owned(),
            JSXAttributeName::NamespacedName(namespaced) => {
                let mut name = namespaced.namespace.name.as_str().to_owned();
                name.push(':');
                name.push_str(namespaced.name.name.as_str());
                name
            }
        },
        JSXAttributeItem::SpreadAttribute(attribute) => {
            source_text(source, attribute.argument.span())
                .unwrap_or_default()
                .to_owned()
        }
    }
}

fn line_fix(source: &str, line: &[JSXAttributeItem<'_>], maximum: usize) -> Option<LintFix> {
    let first = line.first()?.span();
    let last = line.last()?.span();
    let mut replacement = String::new();

    for (index, chunk) in line.chunks(maximum).enumerate() {
        if index > 0 {
            replacement.push('\n');
        }
        for (attribute_index, attribute) in chunk.iter().enumerate() {
            if attribute_index > 0 {
                replacement.push(' ');
            }
            replacement.push_str(source_text(source, attribute.span())?);
        }
    }

    Some(LintFix::replace_range(
        TextRange::new(first.start, last.end),
        replacement,
    ))
}

fn source_text(source: &str, span: Span) -> Option<&str> {
    source.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
}

fn same_line(source: &str, start: u32, end: u32) -> bool {
    source_text(source, Span::new(start, end)).is_some_and(|text| {
        !text
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    })
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the JSX option matrix concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/jsx-max-props-per-line-v5.10.0.json"
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
    }

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_max_props_per_line(source, filename, &options, &mut diagnostics);
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

    fn fixed_to_convergence(source: &str, options: Value) -> String {
        let mut output = source.to_owned();
        for _ in 0..10 {
            let diagnostics = run(&output, Some("fixture.tsx"), options.clone());
            let Some(next) = fixed(&output, &diagnostics) else {
                return output;
            };
            assert_ne!(next, output, "fix pass must make progress");
            output = next;
        }
        panic!("fixes did not converge after ten passes");
    }

    #[test]
    fn replays_every_authored_pinned_upstream_case_exactly() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.valid.len(), 19);
        assert_eq!(fixture.invalid.len(), 22);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .flat_map(|test_case| &test_case.errors)
                .count(),
            22
        );

        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(
                    &test_case.code,
                    Some("fixture.tsx"),
                    test_case.options.clone()
                )
                .is_empty(),
                "valid case {index}: {}",
                test_case.code
            );
        }
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(
                &test_case.code,
                Some("fixture.tsx"),
                test_case.options.clone(),
            );
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
                let reported = source_text(
                    &test_case.code,
                    Span::new(diagnostic.range.start, diagnostic.range.end),
                )
                .expect("reported attribute");
                let prop = expected.data.get("prop").expect("prop data");
                assert!(
                    reported.starts_with(prop) || reported.starts_with('{'),
                    "case {index} must report the first excess prop: {reported}"
                );
            }
            let output = fixed(&test_case.code, &diagnostics);
            assert_eq!(
                output, test_case.output,
                "invalid case {index}: {}",
                test_case.code
            );
            let converged = fixed_to_convergence(&test_case.code, test_case.options.clone());
            assert!(
                run(&converged, Some("fixture.tsx"), test_case.options.clone()).is_empty(),
                "case {index} recursive fixes must converge"
            );
        }
    }

    #[test]
    fn reports_one_diagnostic_per_overfull_physical_line_and_repartitions_each_line() {
        let source = "<Panel one two three\n  four five six seven />";
        let diagnostics = run(
            source,
            Some("fixture.tsx"),
            json!([{ "maximum": { "single": 1, "multi": 2 } }]),
        );
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data.get("prop").map(String::as_str))
                .collect::<Vec<_>>(),
            [Some("three"), Some("six")]
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("<Panel one two\nthree\n  four five\nsix seven />")
        );
    }

    #[test]
    fn uses_single_limit_only_for_an_entirely_single_line_opening_element() {
        let options = json!([{ "maximum": { "single": 1, "multi": 3 } }]);
        assert_eq!(
            run("<Panel one two />", Some("fixture.tsx"), options.clone()).len(),
            1
        );
        assert!(
            run(
                "<Panel one two\n  three />",
                Some("fixture.tsx"),
                options.clone()
            )
            .is_empty()
        );
        assert!(run("<Panel\n  one two three />", Some("fixture.tsx"), options).is_empty());
    }

    #[test]
    fn supports_multiline_only_and_independent_object_limits() {
        assert!(
            run(
                "<Panel one two three />",
                Some("fixture.tsx"),
                json!([{ "maximum": 1, "when": "multiline" }])
            )
            .is_empty()
        );
        assert_eq!(
            run(
                "<Panel one two\n  three four />",
                Some("fixture.tsx"),
                json!([{ "maximum": 1, "when": "multiline" }])
            )
            .len(),
            2
        );
        assert!(
            run(
                "<Panel one two three />",
                Some("fixture.tsx"),
                json!([{ "maximum": { "multi": 1 } }])
            )
            .is_empty()
        );
        assert!(
            run(
                "<Panel\n  one two three />",
                Some("fixture.tsx"),
                json!([{ "maximum": { "single": 1 } }])
            )
            .is_empty()
        );
    }

    #[test]
    fn preserves_exact_unicode_ranges_namespaced_names_and_spread_prop_data() {
        let source =
            "const marker = \"😀\"; const view = <部品 xml:lang=\"日本語\" {...props.値} final />;";
        let diagnostics = run(source, Some("fixture.tsx"), json!([{ "maximum": 1 }]));
        assert_eq!(diagnostics.len(), 1);
        let spread_start = source.find("{...props.値}").expect("spread");
        let spread_end = spread_start + "{...props.値}".len();
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(spread_start as u32, spread_end as u32)
        );
        assert_eq!(
            diagnostics[0].data.get("prop").map(String::as_str),
            Some("props.値")
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some(
                "const marker = \"😀\"; const view = <部品 xml:lang=\"日本語\"\n{...props.値}\nfinal />;"
            )
        );

        let source = "<部品 first xml:lang=\"日本語\" />;";
        let diagnostic = &run(source, Some("fixture.tsx"), json!([{ "maximum": 1 }]))[0];
        assert_eq!(
            diagnostic.data.get("prop").map(String::as_str),
            Some("xml:lang")
        );
    }

    #[test]
    fn supports_crlf_cr_lf_and_ecmascript_unicode_line_terminators() {
        for newline in ["\r\n", "\r", "\n", "\u{2028}", "\u{2029}"] {
            let source = format!("<Panel one two{newline}three four />");
            let diagnostics = run(&source, Some("fixture.tsx"), json!([{ "maximum": 1 }]));
            assert_eq!(diagnostics.len(), 2, "{newline:?}");
            assert_eq!(
                fixed(&source, &diagnostics).as_deref(),
                Some(format!("<Panel one\ntwo{newline}three\nfour />")).as_deref(),
                "{newline:?}"
            );
        }
    }

    #[test]
    fn traverses_nested_jsx_and_typescript_generic_opening_elements_in_source_order() {
        let source = concat!(
            "<Outer one two>",
            "<DataTable<Items> fullscreen keyField=\"id\" items={items} />",
            "</Outer>"
        );
        let diagnostics = run(source, Some("fixture.tsx"), json!([{ "maximum": 1 }]));
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data.get("prop").map(String::as_str))
                .collect::<Vec<_>>(),
            [Some("two"), Some("keyField")]
        );
        let output = fixed(source, &diagnostics).expect("fix");
        assert_eq!(
            output,
            concat!(
                "<Outer one\ntwo>",
                "<DataTable<Items> fullscreen\nkeyField=\"id\"\nitems={items} />",
                "</Outer>"
            )
        );
        assert!(run(&output, Some("fixture.tsx"), json!([{ "maximum": 1 }])).is_empty());
    }

    #[test]
    fn matches_upstream_comment_replacement_semantics() {
        let source = "<Panel one /* dropped */ two three />";
        let diagnostics = run(source, Some("fixture.tsx"), json!([{ "maximum": 1 }]));
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("<Panel one\ntwo\nthree />")
        );
    }

    #[test]
    fn ignores_invalid_jsx_non_jsx_and_non_jsx_filenames_safely() {
        for (source, filename) in [
            ("const comparison = left < right > value;", "fixture.js"),
            ("const text = '<Panel one two />';", "fixture.js"),
            ("const broken = <Panel one two", "fixture.tsx"),
            ("const view = <Panel one={value />;", "fixture.tsx"),
        ] {
            assert!(
                run(source, Some(filename), json!([{ "maximum": 1 }])).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn invalid_option_payloads_are_safe_and_use_stable_defaults() {
        let source = "<Panel one two />";
        for options in [
            Value::Null,
            json!([]),
            json!([42]),
            json!([{ "maximum": 0 }]),
            json!([{ "maximum": "many" }]),
            json!([{ "maximum": 1.5 }]),
        ] {
            assert_eq!(
                run(source, Some("fixture.tsx"), options).len(),
                1,
                "invalid options use maximum 1"
            );
        }
        assert!(
            run(
                source,
                Some("fixture.tsx"),
                json!([{ "maximum": { "single": 0, "multi": "many" } }])
            )
            .is_empty(),
            "invalid independent limits are treated as unspecified/infinite"
        );
    }
}
