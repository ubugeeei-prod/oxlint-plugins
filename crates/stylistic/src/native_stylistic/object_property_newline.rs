//! AST-backed implementation of `@stylistic/object-property-newline`.
//!
//! The upstream rule operates on the exact property/member lists of object
//! expressions, TypeScript type literals, and interface bodies. Oxc provides
//! those lists while the shared stylistic scan supplies comment-aware token
//! boundaries and byte-accurate fixes.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{ObjectExpression, TSInterfaceBody, TSTypeLiteral};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::Scan;

const RULE: &str = "object-property-newline";
const NEWLINE_ID: &str = "propertiesOnNewline";
const NEWLINE_MESSAGE: &str = "Object properties must go on a new line.";
const NEWLINE_ALL_ID: &str = "propertiesOnNewlineAll";
const NEWLINE_ALL_MESSAGE: &str =
    "Object properties must go on a new line if they aren't all on the same line.";

/// Enforces one object property or TypeScript member per line.
pub(crate) fn check_object_property_newline(
    scan: &Scan<'_>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allocator = Allocator::default();
    for source_type in [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        let parsed = Parser::new(&allocator, scan.source(), source_type).parse();
        if parsed.errors.is_empty() {
            let first_diagnostic = diagnostics.len();
            let mut visitor = ObjectPropertyNewlineVisitor {
                scan,
                allow_all_properties_on_same_line: allow_all_properties_on_same_line(options),
                diagnostics,
            };
            visitor.visit_program(&parsed.program);
            diagnostics[first_diagnostic..]
                .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
            return;
        }
    }
}

struct ObjectPropertyNewlineVisitor<'source, 'diagnostics> {
    scan: &'source Scan<'source>,
    allow_all_properties_on_same_line: bool,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for ObjectPropertyNewlineVisitor<'_, '_> {
    fn visit_object_expression(&mut self, node: &ObjectExpression<'ast>) {
        self.check_children(node.properties.iter().map(GetSpan::span));
        walk::walk_object_expression(self, node);
    }

    fn visit_ts_type_literal(&mut self, node: &TSTypeLiteral<'ast>) {
        self.check_children(node.members.iter().map(GetSpan::span));
        walk::walk_ts_type_literal(self, node);
    }

    fn visit_ts_interface_body(&mut self, node: &TSInterfaceBody<'ast>) {
        self.check_children(node.body.iter().map(GetSpan::span));
        walk::walk_ts_interface_body(self, node);
    }
}

impl ObjectPropertyNewlineVisitor<'_, '_> {
    fn check_children(&mut self, children: impl Iterator<Item = Span>) {
        let tokens = children
            .filter_map(|span| self.child_token_bounds(span))
            .collect::<Vec<_>>();

        if self.allow_all_properties_on_same_line && tokens.len() > 1 {
            let first = tokens[0].0;
            let last = tokens[tokens.len() - 1].1;
            if tokens_on_same_line(self.scan, first, last) {
                return;
            }
        }

        for pair in tokens.windows(2) {
            let [(_, previous_last), (current_first, _)] = pair else {
                continue;
            };
            if tokens_on_same_line(self.scan, *previous_last, *current_first) {
                self.report(*current_first);
            }
        }
    }

    fn child_token_bounds(&self, span: Span) -> Option<(usize, usize)> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let mut matches = self.scan.tokens().iter().enumerate().filter(|(_, token)| {
            !token.kind.is_comment() && token.start >= start && token.end <= end
        });
        let first = matches.next()?.0;
        let last = matches.next_back().map_or(first, |(index, _)| index);
        Some((first, last))
    }

    fn report(&mut self, current_first: usize) {
        let tokens = self.scan.tokens();
        let token = tokens[current_first];
        let (message_id, message) = if self.allow_all_properties_on_same_line {
            (NEWLINE_ALL_ID, NEWLINE_ALL_MESSAGE)
        } else {
            (NEWLINE_ID, NEWLINE_MESSAGE)
        };
        let suggestions = self
            .scan
            .prev_significant(current_first)
            .and_then(|separator| {
                let range = tokens[separator].end..token.start;
                self.scan
                    .slice(range.start, range.end)
                    .trim()
                    .is_empty()
                    .then(|| {
                        let range = TextRange::new(
                            u32::try_from(range.start).ok()?,
                            u32::try_from(range.end).ok()?,
                        );
                        Some(LintSuggestion {
                            message_id: message_id.to_owned(),
                            message: message.to_owned(),
                            fixes: std::iter::once(LintFix::replace_range(range, "\n")).collect(),
                        })
                    })?
            })
            .into_iter()
            .collect();
        let (Ok(start), Ok(end)) = (u32::try_from(token.start), u32::try_from(token.end)) else {
            return;
        };

        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range: TextRange::new(start, end),
            suggestions,
        });
    }
}

fn allow_all_properties_on_same_line(options: &Value) -> bool {
    options
        .as_array()
        .and_then(|options| options.first())
        .and_then(Value::as_object)
        .and_then(|option| option.get("allowAllPropertiesOnSameLine"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn tokens_on_same_line(scan: &Scan<'_>, left: usize, right: usize) -> bool {
    !scan
        .slice(scan.tokens()[left].end, scan.tokens()[right].start)
        .chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the upstream option matrix readable in focused tests"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check_object_property_newline(&scan, &options, &mut diagnostics);
        diagnostics
    }

    fn apply_fix_pass(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| suggestion.fixes.iter())
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| (fix.range.start, fix.range.end));

        let mut accepted = Vec::new();
        let mut last_end = None;
        for fix in fixes {
            if last_end.is_some_and(|end| fix.range.start < end) {
                continue;
            }
            last_end = Some(fix.range.end);
            accepted.push(fix);
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

    fn iterative_fixed_output(source: &str, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, options.clone());
            let Some(next) = apply_fix_pass(&output, &diagnostics) else {
                break;
            };
            output = next;
            changed = true;
        }
        changed.then_some(output)
    }

    #[test]
    fn enforces_each_runtime_property_and_preserves_nested_visitor_order() {
        let source =
            "const value = { first: 1, nested: { inner: 1, next: 2 }, method() {}, ...rest };";
        let diagnostics = run(source, json!([]));
        assert_eq!(diagnostics.len(), 4);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message_id == NEWLINE_ID)
        );
        assert_eq!(
            iterative_fixed_output(source, &json!([])).as_deref(),
            Some(
                "const value = { first: 1,\nnested: { inner: 1,\nnext: 2 },\nmethod() {},\n...rest };"
            )
        );
    }

    #[test]
    fn allows_a_wholly_single_line_object_only_when_configured() {
        let options = json!([{ "allowAllPropertiesOnSameLine": true }]);
        assert!(run("const value = { first: 1, second: 2 };", options.clone()).is_empty());

        let diagnostics = run(
            "const value = { first: 1, second: 2,\nthird: 3, fourth: 4 };",
            options,
        );
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message_id == NEWLINE_ALL_ID)
        );
    }

    #[test]
    fn checks_typescript_type_literals_and_interface_bodies() {
        let source = "
type Payload = { id: number; name: string; nested: { left: string; right: string } };
interface Account { id: number; name(): string; [key: string]: unknown }
";
        let diagnostics = run(source, json!([]));
        assert_eq!(diagnostics.len(), 5);
        assert_eq!(
            iterative_fixed_output(source, &json!([])).as_deref(),
            Some(
                "
type Payload = { id: number;
name: string;
nested: { left: string;
right: string } };
interface Account { id: number;
name(): string;
[key: string]: unknown }
"
            )
        );
    }

    #[test]
    fn ignores_patterns_imports_exports_blocks_classes_and_enums() {
        let source = "
const { first, second } = value;
import { first, second } from 'module';
export { first, second };
if (condition) { first(); second(); }
class Example { first = 1; second = 2 }
enum Choice { First, Second }
";
        assert!(run(source, json!([])).is_empty());
    }

    #[test]
    fn comments_between_separator_and_property_suppress_only_the_fix() {
        let source = "const value = { first: 1, /* keep */ second: 2 };";
        let diagnostics = run(source, json!([]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message_id, NEWLINE_ID);
        assert!(diagnostics[0].suggestions.is_empty());
        assert_eq!(iterative_fixed_output(source, &json!([])), None);
    }

    #[test]
    fn handles_every_ecmascript_line_terminator() {
        for linebreak in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("const value = {{ first: 1,{linebreak}second: 2 }};");
            assert!(run(&source, json!([])).is_empty(), "{linebreak:?}");
        }
    }

    #[test]
    fn uses_the_upstream_lf_replacement_for_horizontal_gaps() {
        let source = "const value = { first: 1, \tsecond: 2 };";
        let diagnostics = run(source, json!([]));
        assert_eq!(
            apply_fix_pass(source, &diagnostics).as_deref(),
            Some("const value = { first: 1,\nsecond: 2 };")
        );
    }

    #[test]
    fn preserves_utf8_byte_ranges_and_tsx_object_expressions() {
        let source = "const 日本語 = { 最初: '一', 次: '二' }; const view = <Panel data={{一: 1, 二: 2}} />;";
        let diagnostics = run(source, json!([]));
        assert_eq!(diagnostics.len(), 2);
        let next_start = source.find("次").expect("second Japanese key");
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(next_start as u32, (next_start + "次".len()) as u32)
        );
        assert_eq!(diagnostics[1].message_id, NEWLINE_ID);
    }

    #[test]
    fn invalid_or_empty_option_shapes_fall_back_to_the_default() {
        for options in [json!(null), json!([]), json!([{}]), json!(["invalid"])] {
            assert_eq!(
                run("const value = { first: 1, second: 2 };", options).len(),
                1
            );
        }
    }

    #[test]
    fn parse_failures_do_not_produce_heuristic_diagnostics() {
        assert!(run("const value = { first: 1, second:", json!([])).is_empty());
        assert!(
            run(
                "const view = <Panel data={{ first: 1, second: 2 }",
                json!([])
            )
            .is_empty()
        );
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        suites: Vec<FixtureSuite>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureSuite {
        language: String,
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureCase {
        code: String,
        #[serde(default)]
        options: Value,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
        output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        range: [usize; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [usize; 2],
        text: String,
    }

    #[test]
    fn replays_every_pinned_upstream_case_exactly() {
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/object-property-newline-v5.10.0.json"
        ))
        .expect("object-property-newline fixture must deserialize");

        let mut valid_count = 0;
        let mut invalid_count = 0;
        let mut diagnostic_count = 0;
        for suite in fixture.suites {
            assert!(
                matches!(suite.language.as_str(), "javascript" | "typescript"),
                "unexpected fixture language {}",
                suite.language
            );
            for test in suite.valid {
                valid_count += 1;
                assert!(
                    run(&test.code, test.options).is_empty(),
                    "valid fixture reported: {}",
                    test.code
                );
            }
            for test in suite.invalid {
                invalid_count += 1;
                let diagnostics = run(&test.code, test.options.clone());
                diagnostic_count += diagnostics.len();
                assert_eq!(
                    diagnostics.len(),
                    test.expected_diagnostics.len(),
                    "diagnostic count differs for {}",
                    test.code
                );
                for (actual, expected) in diagnostics.iter().zip(&test.expected_diagnostics) {
                    assert_eq!(actual.message_id, expected.message_id, "{}", test.code);
                    assert_eq!(actual.message, expected.message, "{}", test.code);
                    assert_eq!(
                        [
                            usize::try_from(actual.range.start).expect("range start"),
                            usize::try_from(actual.range.end).expect("range end"),
                        ],
                        expected.range,
                        "{}",
                        test.code
                    );
                    match (&actual.suggestions.first(), &expected.fix) {
                        (Some(suggestion), Some(expected_fix)) => {
                            let actual_fix = suggestion.fixes.first().expect("one upstream fix");
                            assert_eq!(
                                [
                                    usize::try_from(actual_fix.range.start).expect("fix start"),
                                    usize::try_from(actual_fix.range.end).expect("fix end"),
                                ],
                                expected_fix.range,
                                "{}",
                                test.code
                            );
                            assert_eq!(
                                actual_fix.replacement_text, expected_fix.text,
                                "{}",
                                test.code
                            );
                        }
                        (None, None) => {}
                        _ => panic!("fixability differs for {}", test.code),
                    }
                }
                assert_eq!(
                    iterative_fixed_output(&test.code, &test.options),
                    test.output,
                    "recursive fixed output differs for {}",
                    test.code
                );
            }
        }

        assert!(valid_count > 30);
        assert!(invalid_count > 20);
        assert!(diagnostic_count > 30);
    }
}
