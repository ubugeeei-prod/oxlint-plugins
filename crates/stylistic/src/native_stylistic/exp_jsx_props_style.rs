//! Native implementation of experimental
//! `@stylistic/exp-jsx-props-style`.
//!
//! Oxc supplies exact JSX opening-element, generic, attribute, spread, and
//! comment boundaries. The implementation preserves upstream's choice between
//! wrapping and collapsing based on the first prop, including partial fixes
//! when a comment makes one of the whitespace replacements unsafe.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    Comment,
    ast::{JSXAttributeItem, JSXAttributeName, JSXOpeningElement},
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;

const RULE: &str = "exp-jsx-props-style";
const SHOULD_WRAP: &str = "shouldWrap";
const SHOULD_NOT_WRAP: &str = "shouldNotWrap";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Options {
    single_line_max_items: Option<usize>,
    multi_line_min_items: usize,
    multi_line_max_items_per_line: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            single_line_max_items: None,
            multi_line_min_items: 0,
            multi_line_max_items_per_line: 1,
        }
    }
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let Some(configuration) = first_option(options).and_then(Value::as_object) else {
            return Self::default();
        };

        let single_line_max_items = configuration
            .get("singleLine")
            .and_then(Value::as_object)
            .and_then(|single_line| single_line.get("maxItems"))
            .and_then(non_negative_integer);
        let multi_line = configuration.get("multiLine").and_then(Value::as_object);
        let multi_line_min_items = multi_line
            .and_then(|multi_line| multi_line.get("minItems"))
            .and_then(non_negative_integer)
            .unwrap_or(0);
        let multi_line_max_items_per_line = multi_line
            .and_then(|multi_line| multi_line.get("maxItemsPerLine"))
            .and_then(positive_integer)
            .unwrap_or(1);

        Self {
            single_line_max_items,
            multi_line_min_items,
            multi_line_max_items_per_line,
        }
    }
}

fn non_negative_integer(value: &Value) -> Option<usize> {
    value.as_u64().and_then(|value| usize::try_from(value).ok())
}

fn positive_integer(value: &Value) -> Option<usize> {
    non_negative_integer(value).filter(|value| *value > 0)
}

pub(crate) fn check_exp_jsx_props_style(
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

    let mut visitor = PropsStyleVisitor {
        source,
        comments: &parsed.program.comments,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct PropsStyleVisitor<'source, 'comments, 'diagnostics> {
    source: &'source str,
    comments: &'comments [Comment],
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for PropsStyleVisitor<'_, '_, '_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        self.check(element);
        walk::walk_jsx_opening_element(self, element);
    }
}

impl PropsStyleVisitor<'_, '_, '_> {
    fn check(&mut self, element: &JSXOpeningElement<'_>) {
        let Some(first_attribute) = element.attributes.first() else {
            return;
        };

        let single_line = same_line(self.source, element.span.start, element.span.end);
        let first_prop_on_new_line = !same_line(
            self.source,
            element.span.start,
            first_attribute.span().start,
        );
        let need_wrap = if single_line {
            self.options
                .single_line_max_items
                .is_some_and(|maximum| element.attributes.len() > maximum)
        } else {
            element.attributes.len() >= self.options.multi_line_min_items && first_prop_on_new_line
        };
        let maximum_per_line = if need_wrap {
            self.options.multi_line_max_items_per_line
        } else {
            usize::MAX
        };

        let mut items_on_current_line = 0_usize;
        let mut previous_end = element
            .type_arguments
            .as_ref()
            .map_or_else(|| element.name.span().end, |arguments| arguments.span.end);

        for (index, current) in element.attributes.iter().enumerate() {
            let current_span = current.span();
            if same_line(self.source, previous_end, current_span.start) {
                items_on_current_line += 1;
                if need_wrap && (index == 0 || items_on_current_line > maximum_per_line) {
                    self.report(current, previous_end, SHOULD_WRAP, "\n");
                    if index != 0 {
                        items_on_current_line = 1;
                    }
                }
            } else {
                items_on_current_line = 1;
                if !need_wrap {
                    self.report(current, previous_end, SHOULD_NOT_WRAP, " ");
                }
            }
            previous_end = current_span.end;
        }
    }

    fn report(
        &mut self,
        attribute: &JSXAttributeItem<'_>,
        previous_end: u32,
        message_id: &'static str,
        replacement: &'static str,
    ) {
        let prop = prop_name(self.source, attribute);
        let mut message = String::from("Prop `");
        message.push_str(&prop);
        if message_id == SHOULD_WRAP {
            message.push_str("` must be placed on a new line");
        } else {
            message.push_str("` should not be placed on a new line");
        }
        let span = attribute.span();
        let data = BTreeMap::from([("prop".to_owned(), prop)]);
        let suggestions = if self.comments_exist_between(previous_end, span.start) {
            Vec::new()
        } else {
            std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.clone(),
                fixes: std::iter::once(LintFix::replace_range(
                    TextRange::new(previous_end, span.start),
                    replacement,
                ))
                .collect(),
            })
            .collect()
        };
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message,
            data,
            range: TextRange::new(span.start, span.end),
            suggestions,
        });
    }

    fn comments_exist_between(&self, start: u32, end: u32) -> bool {
        self.comments
            .iter()
            .any(|comment| comment.span.start >= start && comment.span.end <= end)
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
        JSXAttributeItem::SpreadAttribute(attribute) => source_text(
            source,
            attribute.argument.span().start,
            attribute.argument.span().end,
        )
        .unwrap_or_default()
        .to_owned(),
    }
}

fn same_line(source: &str, start: u32, end: u32) -> bool {
    source_text(source, start, end).is_some_and(|text| {
        !text
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    })
}

fn source_text(source: &str, start: u32, end: u32) -> Option<&str> {
    source.get(usize::try_from(start).ok()?..usize::try_from(end).ok()?)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the exhaustive JSX option matrix concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/exp-jsx-props-style-v5.10.0.json"
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
        #[serde(rename = "sourceSha256")]
        source_sha256: String,
        #[serde(rename = "ruleSourceSha256")]
        rule_source_sha256: String,
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        fixable_diagnostics: usize,
        unfixable_diagnostics: usize,
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
        recursive_output: String,
        #[serde(default)]
        diagnostics: Vec<ExpectedDiagnostic>,
        #[serde(default)]
        recursive_diagnostics: Vec<ExpectedRecursiveDiagnostic>,
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

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedRecursiveDiagnostic {
        message_id: String,
        message: String,
        range: [u32; 2],
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedFix {
        range: [u32; 2],
        replacement_text: String,
    }

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_exp_jsx_props_style(source, filename, &options, &mut diagnostics);
        diagnostics
    }

    fn fixed(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
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

    fn fixed_to_convergence(source: &str, options: Value) -> (String, Vec<LintDiagnostic>) {
        let mut output = source.to_owned();
        for _ in 0..10 {
            let diagnostics = run(&output, Some("fixture.tsx"), options.clone());
            let Some(next) = fixed(&output, &diagnostics) else {
                return (output, diagnostics);
            };
            assert_ne!(next, output, "fix pass must make progress");
            output = next;
        }
        panic!("fixes did not converge after ten passes");
    }

    #[test]
    fn replays_every_authored_pinned_upstream_case_exactly() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(
            fixture.generated.source_sha256,
            "926f8805c068941c3ae2d959c180aae14da09e55d2c1eefabef45081ea0f602a"
        );
        assert_eq!(
            fixture.generated.rule_source_sha256,
            "c3dbc6b2026f5ec5c25677d27ae9be0bd78752d691722049fd0602fd9c12a063"
        );
        assert_eq!(fixture.generated.inventory.valid, 17);
        assert_eq!(fixture.generated.inventory.invalid, 11);
        assert_eq!(fixture.generated.inventory.diagnostics, 17);
        assert_eq!(fixture.generated.inventory.fixable_diagnostics, 15);
        assert_eq!(fixture.generated.inventory.unfixable_diagnostics, 2);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 11);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 0);
        assert_eq!(fixture.generated.inventory.total, 28);
        assert_eq!(fixture.valid.len(), 17);
        assert_eq!(fixture.invalid.len(), 11);

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
                test_case.diagnostics.len(),
                "invalid case {index}: {}",
                test_case.code
            );
            for (diagnostic, expected) in diagnostics.iter().zip(&test_case.diagnostics) {
                assert_eq!(diagnostic.message_id, expected.message_id, "case {index}");
                assert_eq!(diagnostic.message, expected.message, "case {index}");
                assert_eq!(diagnostic.data, expected.data, "case {index}");
                assert_eq!(
                    diagnostic.range,
                    TextRange::new(expected.range[0], expected.range[1]),
                    "case {index}"
                );
                match (&diagnostic.suggestions.first(), &expected.fix) {
                    (Some(suggestion), Some(expected_fix)) => {
                        assert_eq!(suggestion.message_id, expected.message_id, "case {index}");
                        assert_eq!(suggestion.message, expected.message, "case {index}");
                        assert_eq!(suggestion.fixes.len(), 1, "case {index}");
                        assert_eq!(
                            suggestion.fixes[0].range,
                            TextRange::new(expected_fix.range[0], expected_fix.range[1]),
                            "case {index}"
                        );
                        assert_eq!(
                            suggestion.fixes[0].replacement_text, expected_fix.replacement_text,
                            "case {index}"
                        );
                    }
                    (None, None) => {}
                    _ => panic!("case {index} fixability mismatch"),
                }
            }
            assert_eq!(
                fixed(&test_case.code, &diagnostics),
                test_case.output,
                "invalid case {index}: {}",
                test_case.code
            );
            let (recursive_output, recursive_diagnostics) =
                fixed_to_convergence(&test_case.code, test_case.options.clone());
            assert_eq!(
                recursive_output, test_case.recursive_output,
                "case {index} recursive output"
            );
            assert_eq!(
                recursive_diagnostics.len(),
                test_case.recursive_diagnostics.len(),
                "case {index} recursive diagnostics"
            );
            for (diagnostic, expected) in recursive_diagnostics
                .iter()
                .zip(&test_case.recursive_diagnostics)
            {
                assert_eq!(diagnostic.message_id, expected.message_id, "case {index}");
                assert_eq!(diagnostic.message, expected.message, "case {index}");
                assert_eq!(
                    diagnostic.range,
                    TextRange::new(expected.range[0], expected.range[1]),
                    "case {index}"
                );
            }
        }
    }

    #[test]
    fn covers_every_option_and_first_prop_decision_path() {
        assert!(run("<App one two />", Some("fixture.tsx"), Value::Null).is_empty());
        assert_eq!(
            run(
                "<App one two />",
                Some("fixture.tsx"),
                json!([{ "singleLine": { "maxItems": 1 } }])
            )
            .len(),
            2
        );
        assert_eq!(
            fixed(
                "<App one two three four />",
                &run(
                    "<App one two three four />",
                    Some("fixture.tsx"),
                    json!([{
                        "singleLine": { "maxItems": 3 },
                        "multiLine": { "maxItemsPerLine": 2 }
                    }])
                )
            )
            .as_deref(),
            Some("<App\none two\nthree four />")
        );
        assert_eq!(
            fixed(
                "<App\n one\n two />",
                &run(
                    "<App\n one\n two />",
                    Some("fixture.tsx"),
                    json!([{ "multiLine": { "minItems": 3 } }])
                )
            )
            .as_deref(),
            Some("<App one two />")
        );
        assert_eq!(
            fixed(
                "<App\n one two three\n four five />",
                &run(
                    "<App\n one two three\n four five />",
                    Some("fixture.tsx"),
                    json!([{ "multiLine": { "minItems": 1, "maxItemsPerLine": 2 } }])
                )
            )
            .as_deref(),
            Some("<App\n one two\nthree\n four five />")
        );
        assert_eq!(
            run(
                "<App\n one two />",
                Some("fixture.tsx"),
                json!([{ "multiLine": { "minItems": 0, "maxItemsPerLine": 2 } }])
            )
            .len(),
            0
        );
    }

    #[test]
    fn preserves_unicode_namespaced_and_spread_prop_data_with_exact_byte_ranges() {
        let source =
            "const marker = \"😀\"; const view = <部品 xml:lang=\"日本語\" {...props.値} final />;";
        let options = json!([{ "singleLine": { "maxItems": 1 } }]);
        let diagnostics = run(source, Some("fixture.tsx"), options);
        assert_eq!(diagnostics.len(), 3);
        let first_start = source.find("xml:lang").expect("namespaced prop");
        let spread_start = source.find("{...props.値}").expect("spread prop");
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(
                first_start as u32,
                (first_start + "xml:lang=\"日本語\"".len()) as u32
            )
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data.get("prop").map(String::as_str))
                .collect::<Vec<_>>(),
            [Some("xml:lang"), Some("props.値"), Some("final")]
        );
        assert_eq!(
            diagnostics[1].range,
            TextRange::new(
                spread_start as u32,
                (spread_start + "{...props.値}".len()) as u32
            )
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some(
                "const marker = \"😀\"; const view = <部品\nxml:lang=\"日本語\"\n{...props.値}\nfinal />;"
            )
        );
    }

    #[test]
    fn handles_every_ecmascript_line_terminator_for_wrap_and_collapse() {
        for newline in ["\r\n", "\r", "\n", "\u{2028}", "\u{2029}"] {
            let wrap_source = format!("<Panel{newline}one two />");
            let wrap_diagnostics = run(&wrap_source, Some("fixture.tsx"), Value::Null);
            assert_eq!(wrap_diagnostics.len(), 1, "{newline:?}");
            assert_eq!(
                fixed(&wrap_source, &wrap_diagnostics).as_deref(),
                Some(format!("<Panel{newline}one\ntwo />")).as_deref(),
                "{newline:?}"
            );

            let collapse_source = format!("<Panel{newline}one{newline}two />");
            let collapse_options = json!([{ "multiLine": { "minItems": 3 } }]);
            let collapse_diagnostics = run(&collapse_source, Some("fixture.tsx"), collapse_options);
            assert_eq!(collapse_diagnostics.len(), 2, "{newline:?}");
            assert_eq!(
                fixed(&collapse_source, &collapse_diagnostics).as_deref(),
                Some("<Panel one two />"),
                "{newline:?}"
            );
        }
    }

    #[test]
    fn preserves_comments_and_reports_unfixable_boundaries() {
        let source = "<App foo /* keep */ bar baz />";
        let diagnostics = run(
            source,
            Some("fixture.tsx"),
            json!([{ "singleLine": { "maxItems": 1 } }]),
        );
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.suggestions.len())
                .collect::<Vec<_>>(),
            [1, 0, 1]
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("<App\nfoo /* keep */ bar\nbaz />")
        );
    }

    #[test]
    fn handles_nested_tsx_generics_in_physical_source_order() {
        let source = concat!(
            "<Outer first child={<DataTable<Row> one two />} third\n",
            "fourth />"
        );
        let diagnostics = run(
            source,
            Some("fixture.tsx"),
            json!([{ "singleLine": { "maxItems": 1 } }]),
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.data.get("prop").map(String::as_str))
                .collect::<Vec<_>>(),
            [Some("one"), Some("two"), Some("fourth")]
        );
        assert!(
            diagnostics
                .windows(2)
                .all(|pair| pair[0].range.start <= pair[1].range.start)
        );
    }

    #[test]
    fn rejects_malformed_or_non_jsx_sources_and_uses_safe_option_defaults() {
        for (source, filename) in [
            ("const comparison = left < right > value;", "fixture.js"),
            ("const text = '<Panel one two />';", "fixture.js"),
            ("const broken = <Panel one two", "fixture.tsx"),
            ("const view = <Panel one={value />;", "fixture.tsx"),
        ] {
            assert!(
                run(
                    source,
                    Some(filename),
                    json!([{ "singleLine": { "maxItems": 0 } }])
                )
                .is_empty(),
                "{source}"
            );
        }

        for options in [
            Value::Null,
            json!([]),
            json!([42]),
            json!([{ "singleLine": { "maxItems": -1 } }]),
            json!([{ "singleLine": { "maxItems": "many" } }]),
            json!([{ "multiLine": { "minItems": -1, "maxItemsPerLine": 0 } }]),
        ] {
            assert!(
                run("<Panel one two />", Some("fixture.tsx"), options).is_empty(),
                "invalid options use stable defaults"
            );
        }
        assert_eq!(
            run(
                "<Panel\none two />",
                None,
                json!([{ "multiLine": { "maxItemsPerLine": 1 } }])
            )
            .len(),
            1
        );
    }
}
