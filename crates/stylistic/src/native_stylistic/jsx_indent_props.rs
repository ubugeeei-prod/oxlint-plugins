//! Native implementation of stable `@stylistic/jsx-indent-props` v5.10.0.
//!
//! Oxc supplies exact JSX opening-element and attribute spans. This port keeps
//! upstream's stateful ternary adjustment, `first`/`tab`/integer modes,
//! first-token-on-line semantics, full-attribute reports, and line-prefix
//! replacement fixes.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::JSXOpeningElement;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;

const RULE: &str = "jsx-indent-props";
const WRONG_INDENT: &str = "wrongIndent";
const DEFAULT_INDENT: i64 = 4;
const MAX_SAFE_FIX_INDENT: i64 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IndentMode {
    Spaces(i64),
    Tabs,
    First,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Options {
    indent_mode: IndentMode,
    ignore_ternary_operator: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            indent_mode: IndentMode::Spaces(DEFAULT_INDENT),
            ignore_ternary_operator: false,
        }
    }
}

impl Options {
    fn from_value(options: &Value) -> Self {
        let Some(option) = first_option(options) else {
            return Self::default();
        };
        if let Some(object) = option.as_object() {
            return Self {
                indent_mode: object
                    .get("indentMode")
                    .and_then(parse_indent_mode)
                    .unwrap_or(IndentMode::Spaces(DEFAULT_INDENT)),
                ignore_ternary_operator: object
                    .get("ignoreTernaryOperator")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            };
        }
        Self {
            indent_mode: parse_indent_mode(option).unwrap_or(IndentMode::Spaces(DEFAULT_INDENT)),
            ignore_ternary_operator: false,
        }
    }

    fn indent_type(self) -> &'static str {
        if self.indent_mode == IndentMode::Tabs {
            "tab"
        } else {
            "space"
        }
    }

    fn indent_character(self) -> char {
        if self.indent_mode == IndentMode::Tabs {
            '\t'
        } else {
            ' '
        }
    }

    fn indent_size(self) -> Option<i64> {
        match self.indent_mode {
            IndentMode::Spaces(size) => Some(size),
            IndentMode::Tabs => Some(1),
            IndentMode::First => None,
        }
    }
}

fn parse_indent_mode(value: &Value) -> Option<IndentMode> {
    match value {
        Value::String(value) if value == "tab" => Some(IndentMode::Tabs),
        Value::String(value) if value == "first" => Some(IndentMode::First),
        Value::Number(value) => value.as_i64().map(IndentMode::Spaces),
        _ => None,
    }
}

pub(crate) fn check_jsx_indent_props(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_value(options);
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

    let mut visitor = IndentPropsVisitor {
        source,
        options,
        line_is_using_operator: false,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct IndentPropsVisitor<'source, 'diagnostics> {
    source: &'source str,
    options: Options,
    line_is_using_operator: bool,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for IndentPropsVisitor<'_, '_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        // Upstream checks the opening element on entry. That means all outer
        // attributes are reported before JSX nested inside an attribute value.
        self.check(element);
        walk::walk_jsx_opening_element(self, element);
    }
}

impl IndentPropsVisitor<'_, '_> {
    fn check(&mut self, element: &JSXOpeningElement<'_>) {
        let Some(first_attribute) = element.attributes.first() else {
            return;
        };

        let mut needed = match self.options.indent_mode {
            IndentMode::First => utf16_column(self.source, first_attribute.span().start),
            IndentMode::Spaces(size) => self.get_node_indent(element.span.start, true) + size,
            IndentMode::Tabs => self.get_node_indent(element.span.start, true) + 1,
        };
        let mut previous = element
            .type_arguments
            .as_ref()
            .map_or_else(|| element.name.span(), |arguments| arguments.span);

        for attribute in &element.attributes {
            let span = attribute.span();
            let gotten = self.get_node_indent(span.start, false);
            let current_operator = line_starts_with_operator(self.source, span.start);
            if self.line_is_using_operator
                && !current_operator
                && self.options.indent_mode != IndentMode::First
                && !self.options.ignore_ternary_operator
            {
                needed += self.options.indent_size().unwrap_or_default();
                self.line_is_using_operator = false;
            }

            if gotten != needed && is_first_token_in_line(self.source, previous.end, span.start) {
                self.report(span, needed, gotten);
            }
            previous = span;
        }
    }

    fn get_node_indent(&mut self, offset: u32, opening_element: bool) -> i64 {
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let line_start = line_start(self.source, start);
        let prefix_end =
            if opening_element && self.source.as_bytes().get(start).copied() == Some(b'<') {
                start.saturating_add(1)
            } else {
                start
            }
            .min(self.source.len());
        let prefix = self.source.get(line_start..prefix_end).unwrap_or_default();
        let current_operator = prefix
            .trim_start_matches([' ', '\t'])
            .starts_with([':', '?']);
        if current_operator {
            self.line_is_using_operator = true;
        } else if prefix.contains('<') {
            self.line_is_using_operator = false;
        }

        self.source
            .get(line_start..)
            .unwrap_or_default()
            .chars()
            .take_while(|character| *character == self.options.indent_character())
            .count() as i64
    }

    #[allow(
        clippy::disallowed_macros,
        clippy::disallowed_methods,
        reason = "Rendered diagnostic strings and interpolation data are explicit JavaScript ABI boundary allocations"
    )]
    fn report(&mut self, span: Span, needed: i64, gotten: i64) {
        let indent_type = self.options.indent_type();
        let characters = if needed == 1 {
            "character"
        } else {
            "characters"
        };
        let message = format!(
            "Expected indentation of {needed} {indent_type} {characters} but found {gotten}."
        );
        let data = BTreeMap::from([
            ("needed".to_owned(), needed.to_string()),
            ("type".to_owned(), indent_type.to_owned()),
            ("characters".to_owned(), characters.to_owned()),
            ("gotten".to_owned(), gotten.to_string()),
        ]);
        let suggestions = if (0..=MAX_SAFE_FIX_INDENT).contains(&needed) {
            let replacement = self
                .options
                .indent_character()
                .to_string()
                .repeat(needed as usize);
            std::iter::once(LintSuggestion {
                message_id: WRONG_INDENT.to_owned(),
                message: message.clone(),
                fixes: std::iter::once(LintFix::replace_range(
                    TextRange::new(line_start_u32(self.source, span.start), span.start),
                    replacement,
                ))
                .collect(),
            })
            .collect()
        } else {
            Vec::new()
        };
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: WRONG_INDENT.to_owned(),
            message,
            data,
            range: TextRange::new(span.start, span.end),
            suggestions,
        });
    }
}

fn line_starts_with_operator(source: &str, offset: u32) -> bool {
    let start = usize::try_from(offset).unwrap_or(usize::MAX);
    let beginning = line_start(source, start);
    source
        .get(beginning..start.min(source.len()))
        .unwrap_or_default()
        .trim_start_matches([' ', '\t'])
        .starts_with([':', '?'])
}

fn is_first_token_in_line(source: &str, previous_end: u32, current_start: u32) -> bool {
    let start = usize::try_from(previous_end).unwrap_or(usize::MAX);
    let end = usize::try_from(current_start).unwrap_or(usize::MAX);
    source
        .get(start.min(source.len())..end.min(source.len()))
        .is_some_and(|gap| gap.chars().any(is_line_terminator))
}

fn utf16_column(source: &str, offset: u32) -> i64 {
    let offset = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(source.len());
    let start = line_start(source, offset);
    source
        .get(start..offset)
        .map_or(0, |prefix| prefix.encode_utf16().count() as i64)
}

fn line_start_u32(source: &str, offset: u32) -> u32 {
    u32::try_from(line_start(
        source,
        usize::try_from(offset).unwrap_or(usize::MAX),
    ))
    .unwrap_or(u32::MAX)
}

fn line_start(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    source
        .get(..offset)
        .and_then(|prefix| {
            prefix.char_indices().rev().find_map(|(index, character)| {
                is_line_terminator(character).then_some(index + character.len_utf8())
            })
        })
        .unwrap_or(0)
}

fn is_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    reason = "serde_json::json keeps the exhaustive option and JSX edge matrices readable"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<Case>,
        invalid: Vec<Case>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Generated {
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
        unfixable_invalid: usize,
        total: usize,
        fixable_invalid: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        code: String,
        options: Value,
        parser: String,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        recursive_output: Option<String>,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        data: ExpectedData,
        range: [u32; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    struct ExpectedData {
        needed: i64,
        #[serde(rename = "type")]
        indent_type: String,
        characters: String,
        gotten: i64,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-indent-props-v5.10.0.json"
        ))
        .expect("generated jsx-indent-props fixture is valid JSON")
    }

    fn filename(parser: &str) -> &'static str {
        if parser == "typescript" {
            "fixture.tsx"
        } else {
            "fixture.jsx"
        }
    }

    fn run(source: &str, filename: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_indent_props(source, Some(filename), options, &mut diagnostics);
        diagnostics
    }

    fn apply(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
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
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    fn recursive(source: &str, filename: &str, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, filename, options);
            let Some(next) = apply(&output, &diagnostics) else {
                return changed.then_some(output);
            };
            if next == output {
                return changed.then_some(output);
            }
            output = next;
            changed = true;
        }
        panic!("jsx-indent-props fixes did not converge");
    }

    #[test]
    fn replays_every_pinned_parser_expanded_case_exactly() {
        let fixture = fixture();
        assert_eq!(fixture.generated.inventory.logical_valid, 23);
        assert_eq!(fixture.generated.inventory.logical_invalid, 17);
        assert_eq!(fixture.generated.inventory.valid, 46);
        assert_eq!(fixture.generated.inventory.invalid, 34);
        assert_eq!(fixture.generated.inventory.diagnostics, 42);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 0);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 34);
        assert_eq!(fixture.generated.inventory.total, 80);

        for (index, case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&case.code, filename(&case.parser), &case.options).is_empty(),
                "valid case {index} reported diagnostics"
            );
        }
        for (index, case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&case.code, filename(&case.parser), &case.options);
            assert_eq!(
                diagnostics.len(),
                case.expected_diagnostics.len(),
                "invalid case {index} diagnostic count"
            );
            for (actual, expected) in diagnostics.iter().zip(&case.expected_diagnostics) {
                assert_eq!(
                    actual.message_id, expected.message_id,
                    "invalid case {index}"
                );
                assert_eq!(actual.message, expected.message, "invalid case {index}");
                assert_eq!(
                    actual.data,
                    BTreeMap::from([
                        ("needed".to_owned(), expected.data.needed.to_string()),
                        ("type".to_owned(), expected.data.indent_type.clone()),
                        ("characters".to_owned(), expected.data.characters.clone()),
                        ("gotten".to_owned(), expected.data.gotten.to_string()),
                    ]),
                    "invalid case {index}"
                );
                assert_eq!(
                    actual.range,
                    TextRange::new(expected.range[0], expected.range[1]),
                    "invalid case {index}"
                );
                let actual_fix = actual
                    .suggestions
                    .first()
                    .and_then(|suggestion| suggestion.fixes.first());
                match (&expected.fix, actual_fix) {
                    (Some(expected), Some(actual)) => {
                        assert_eq!(
                            actual.range,
                            TextRange::new(expected.range[0], expected.range[1]),
                            "invalid case {index}"
                        );
                        assert_eq!(
                            actual.replacement_text, expected.text,
                            "invalid case {index}"
                        );
                    }
                    (None, None) => {}
                    _ => panic!("invalid case {index} fix presence differed"),
                }
            }
            assert_eq!(
                apply(&case.code, &diagnostics),
                case.output,
                "invalid case {index} first-pass output"
            );
            assert_eq!(
                recursive(&case.code, filename(&case.parser), &case.options),
                case.recursive_output,
                "invalid case {index} recursive output"
            );
        }
    }

    #[test]
    fn covers_integer_tab_first_and_ternary_modes() {
        assert!(run("<App\n  prop />", "fixture.tsx", &json!([2])).is_empty());
        assert!(run("\t<App\n\t\tprop />", "fixture.tsx", &json!(["tab"])).is_empty());
        assert!(
            run(
                "const view = <App first\n                  second />;",
                "fixture.tsx",
                &json!(["first"])
            )
            .is_empty()
        );
        assert!(
            run(
                "const view = ready\n  ? <App\n    prop />\n  : null;",
                "fixture.tsx",
                &json!([{ "indentMode": 2, "ignoreTernaryOperator": true }])
            )
            .is_empty()
        );
        assert_eq!(
            run(
                "const view = ready\n  ? <App\n    prop />\n  : null;",
                "fixture.tsx",
                &json!([{ "indentMode": 2, "ignoreTernaryOperator": false }])
            )[0]
            .data["needed"],
            "6"
        );
    }

    #[test]
    fn preserves_outer_before_nested_source_order() {
        let source = "<Outer\nbad={<Inner\nx />}\nlater />;";
        let diagnostics = run(source, "fixture.tsx", &json!([2]));
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range.start)
                .collect::<Vec<_>>(),
            [
                source.find("bad=").unwrap() as u32,
                source.find("later").unwrap() as u32,
                source.find("x />").unwrap() as u32,
            ]
        );
    }

    #[test]
    fn handles_unicode_tsx_generics_and_first_mode_columns() {
        let source = "const 絵 = <外.部<型> 最初\n                  次 />;";
        let diagnostics = run(source, "fixture.tsx", &json!(["first"]));
        assert!(diagnostics.is_empty());

        let invalid = "const emoji = \"😀\"; const view = <App<型> first\n  second />;";
        let diagnostics = run(invalid, "fixture.tsx", &json!(["first"]));
        assert_eq!(diagnostics.len(), 1);
        let second = invalid.find("second").unwrap();
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(second as u32, (second + "second".len()) as u32)
        );
        assert_eq!(diagnostics[0].data["needed"], "41");
    }

    #[test]
    fn supports_every_ecmascript_line_terminator() {
        for separator in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let source = format!("<App{separator}prop />");
            let diagnostics = run(&source, "fixture.tsx", &Value::Null);
            assert_eq!(diagnostics.len(), 1, "{separator:?}");
            assert_eq!(diagnostics[0].data["needed"], "4", "{separator:?}");
            assert_eq!(
                apply(&source, &diagnostics),
                Some(format!("<App{separator}    prop />")),
                "{separator:?}"
            );
        }
    }

    #[test]
    fn preserves_comments_spreads_namespaces_members_and_source_boundaries() {
        let source = "<svg:path\n/* keep */\nfoo\n{...props} />;\n<UI.Button\nbar />;";
        let diagnostics = run(source, "fixture.tsx", &json!([2]));
        assert_eq!(diagnostics.len(), 3);
        let output = apply(source, &diagnostics).unwrap();
        assert!(output.contains("/* keep */"));
        assert!(output.contains("\n  foo\n  {...props}"));
        assert!(output.contains("<UI.Button\n  bar"));
        assert!(run("<></>", "fixture.tsx", &Value::Null).is_empty());
    }

    #[test]
    fn malformed_sources_and_options_fail_safely() {
        assert!(run("<App>", "fixture.tsx", &Value::Null).is_empty());
        assert_eq!(
            run("<App\nx />", "fixture.tsx", &json!(["invalid"]))[0].data["needed"],
            "4"
        );
        assert_eq!(
            run(
                "<App\nx />",
                "fixture.tsx",
                &json!([{ "indentMode": null, "ignoreTernaryOperator": "yes" }])
            )[0]
            .data["needed"],
            "4"
        );
        let huge = run(
            "<App\nx />",
            "fixture.tsx",
            &json!([9_223_372_036_854_775_807_i64]),
        );
        assert_eq!(huge.len(), 1);
        assert!(huge[0].suggestions.is_empty());
    }
}
