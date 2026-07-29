//! Native implementation of stable `@stylistic/jsx-newline`.
//!
//! The upstream rule examines direct JSX children after traversal. Oxc gives us
//! the same child ordering and exact byte spans, so the native implementation
//! checks each element or fragment in pre-order and reports the following JSX
//! child with the upstream whitespace-node replacement.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXChild, JSXElement, JSXFragment};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::option_object_bool;

const RULE: &str = "jsx-newline";
const REQUIRE: &str = "JSX element should start in a new line";
const PREVENT: &str = "JSX element should not start in a new line";
const ALLOW_MULTILINES: &str = "Multiline JSX elements should start in a new line";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Options {
    prevent: bool,
    allow_multilines: bool,
}

impl Options {
    fn from_value(value: &Value) -> Self {
        Self {
            prevent: option_object_bool(value, "prevent", false),
            allow_multilines: option_object_bool(value, "allowMultilines", false),
        }
    }
}

pub(crate) fn check_jsx_newline(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_value(options);
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

    let mut visitor = JsxNewlineVisitor {
        source,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct JsxNewlineVisitor<'source, 'diagnostics> {
    source: &'source str,
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxNewlineVisitor<'_, '_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'ast>) {
        self.check_children(&element.children);
        walk::walk_jsx_element(self, element);
    }

    fn visit_jsx_fragment(&mut self, fragment: &JSXFragment<'ast>) {
        self.check_children(&fragment.children);
        walk::walk_jsx_fragment(self, fragment);
    }
}

impl JsxNewlineVisitor<'_, '_> {
    fn check_children(&mut self, children: &[JSXChild<'_>]) {
        if !children.iter().any(is_candidate) {
            return;
        }

        for (index, child) in children.iter().enumerate() {
            if !is_candidate(child) || is_block_comment(self.source, child) {
                continue;
            }
            let Some(JSXChild::Text(whitespace)) = children.get(index + 1) else {
                continue;
            };
            let Some(reported_child) = children.get(index + 2) else {
                continue;
            };

            // Upstream intentionally uses `/\n\s*\n/` against JSXText.value:
            // CR, LS, and PS count for `\s`, but both boundary terminators must
            // be LF.
            let without_blank_line = !has_lf_blank_line(whitespace.value.as_str());
            let next_non_comment = children[index + 2..].iter().find(|candidate| {
                is_candidate(candidate) && !is_block_comment(self.source, candidate)
            });
            let allow_multiline_report = self.options.allow_multilines
                && (is_multiline(self.source, child.span())
                    || next_non_comment
                        .is_some_and(|candidate| is_multiline(self.source, candidate.span())));

            if allow_multiline_report {
                if without_blank_line {
                    self.report(
                        reported_child.span(),
                        whitespace.span,
                        "allowMultilines",
                        ALLOW_MULTILINES,
                        replace_last_lf_group(source_text(self.source, whitespace.span), false),
                    );
                }
                continue;
            }

            if without_blank_line == self.options.prevent {
                continue;
            }
            let (message_id, message) = if self.options.prevent {
                ("prevent", PREVENT)
            } else {
                ("require", REQUIRE)
            };
            self.report(
                reported_child.span(),
                whitespace.span,
                message_id,
                message,
                replace_last_lf_group(
                    source_text(self.source, whitespace.span),
                    self.options.prevent,
                ),
            );
        }
    }

    fn report(
        &mut self,
        report_span: Span,
        whitespace_span: Span,
        message_id: &'static str,
        message: &'static str,
        replacement: String,
    ) {
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range: TextRange::new(report_span.start, report_span.end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(
                    TextRange::new(whitespace_span.start, whitespace_span.end),
                    replacement,
                ))
                .collect(),
            })
            .collect(),
        });
    }
}

fn is_candidate(child: &JSXChild<'_>) -> bool {
    matches!(
        child,
        JSXChild::Element(_) | JSXChild::ExpressionContainer(_)
    )
}

fn is_block_comment(source: &str, child: &JSXChild<'_>) -> bool {
    source_text(source, child.span())
        .trim_start_matches(is_ecmascript_whitespace)
        .starts_with("{/*")
}

fn has_lf_blank_line(text: &str) -> bool {
    let bytes = text.as_bytes();
    for first_lf in bytes
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (*byte == b'\n').then_some(index))
    {
        let mut cursor = first_lf + 1;
        while cursor < text.len() {
            let Some(character) = text[cursor..].chars().next() else {
                break;
            };
            if character == '\n' {
                return true;
            }
            if !is_ecmascript_whitespace(character) {
                break;
            }
            cursor += character.len_utf8();
        }
    }
    false
}

fn is_multiline(source: &str, span: Span) -> bool {
    source_text(source, span)
        .chars()
        .any(is_ecmascript_line_terminator)
}

fn source_text(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or_default()
}

/// Applies upstream's `/(\n)(?!.*\1)/g` or
/// `/(\n\n)(?!.*\1)/g` replacement. JavaScript's dot does not cross any
/// ECMAScript line terminator, which matters for mixed CR/LF/LS/PS input.
fn replace_last_lf_group(text: &str, prevent: bool) -> String {
    let target = if prevent { "\n\n" } else { "\n" };
    let replacement = if prevent { "\n" } else { "\n\n" };
    let mut output = String::with_capacity(text.len() + usize::from(!prevent));
    let mut cursor = 0;

    while let Some(relative) = text[cursor..].find(target) {
        let start = cursor + relative;
        let end = start + target.len();
        output.push_str(&text[cursor..start]);
        if next_line_terminator_matches(text, end, target) {
            output.push_str(target);
        } else {
            output.push_str(replacement);
        }
        cursor = end;
    }
    output.push_str(&text[cursor..]);
    output
}

fn next_line_terminator_matches(text: &str, start: usize, target: &str) -> bool {
    let mut cursor = start;
    while cursor < text.len() {
        let Some(character) = text[cursor..].chars().next() else {
            return false;
        };
        if is_ecmascript_line_terminator(character) {
            return text[cursor..].starts_with(target);
        }
        cursor += character.len_utf8();
    }
    false
}

fn is_ecmascript_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
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
    reason = "serde_json::json keeps the compatibility option matrix concise"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_newline(source, filename, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(source: &str, options: Value) -> Vec<String> {
        run(source, Some("fixture.tsx"), options)
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    fn apply(source: &str, diagnostics: &[LintDiagnostic]) -> String {
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        output
    }

    fn upstream_fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-newline-v5.10.0.json"
        ))
        .expect("generated jsx-newline fixture is valid JSON")
    }

    #[test]
    fn supports_default_prevent_and_allow_multilines() {
        assert_eq!(ids("<A><B />\n<C /></A>", Value::Null), ["require"]);
        assert_eq!(
            ids("<A><B />\n\n<C /></A>", json!([{ "prevent": true }])),
            ["prevent"]
        );
        assert_eq!(
            ids(
                "<A><B />\n<C\n  prop /></A>",
                json!([{ "prevent": true, "allowMultilines": true }])
            ),
            ["allowMultilines"]
        );
        assert!(ids("<A><B />\n<C /></A>", json!([{ "prevent": true }])).is_empty());
    }

    #[test]
    fn reports_nested_parents_in_eslint_source_order() {
        let source = "<A><B><C />\n<D /></B>\n<E><F />\n<G /></E></A>";
        let diagnostics = run(source, Some("fixture.jsx"), Value::Null);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| source_text(
                    source,
                    Span::new(diagnostic.range.start, diagnostic.range.end)
                ))
                .collect::<Vec<_>>(),
            ["<D />", "<E><F />\n<G /></E>", "<G />"]
        );
        assert_eq!(
            apply(source, &diagnostics),
            "<A><B><C />\n\n<D /></B>\n\n<E><F />\n\n<G /></E></A>"
        );
    }

    #[test]
    fn skips_block_comment_expression_containers() {
        let source = "<A>{/* first */}\n<B />\n{/* second */}\n<C /></A>";
        let diagnostics = run(source, Some("fixture.jsx"), Value::Null);
        assert_eq!(ids(source, Value::Null), ["require"]);
        assert_eq!(
            source_text(
                source,
                Span::new(diagnostics[0].range.start, diagnostics[0].range.end)
            ),
            "{/* second */}"
        );
    }

    #[test]
    fn preserves_lf_only_blank_line_detection_and_ecmascript_multiline_locations() {
        assert_eq!(
            ids("<A><B />\n\u{2028}\n<C /></A>", Value::Null),
            Vec::<String>::new()
        );
        assert_eq!(
            ids(
                "<A><B\r  prop />\r<C /></A>",
                json!([{ "prevent": true, "allowMultilines": true }])
            ),
            ["allowMultilines"]
        );
        assert_eq!(
            ids("<A><B />\r\n\r\n<C /></A>", json!([{ "prevent": true }])),
            ["prevent"]
        );
    }

    #[test]
    fn handles_unicode_byte_ranges_tsx_and_invalid_inputs() {
        let source = "const 絵: JSX.Element = <外><内 />\n<次 /></外>;";
        let diagnostics = run(source, Some("fixture.tsx"), Value::Null);
        assert_eq!(diagnostics.len(), 1);
        let start = source.find("<次").expect("second JSX child");
        let end = start + "<次 />".len();
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(start as u32, end as u32)
        );
        assert_eq!(
            apply(source, &diagnostics),
            source.replacen("\n<次", "\n\n<次", 1)
        );

        assert!(run("const broken = <A>", Some("fixture.tsx"), Value::Null).is_empty());
        assert!(run("const value = 1;", Some("fixture.js"), json!([42])).is_empty());
        assert_eq!(
            ids("<A><B />\n<C /></A>", json!([{ "prevent": "yes" }])),
            ["require"]
        );
    }

    #[test]
    fn exactly_reproduces_js_last_lf_replacements() {
        assert_eq!(replace_last_lf_group("\n  ", false), "\n\n  ");
        assert_eq!(replace_last_lf_group("\n\n  ", false), "\n\n\n  ");
        assert_eq!(replace_last_lf_group("\n\n  ", true), "\n  ");
        assert_eq!(replace_last_lf_group("\n\r\n  ", false), "\n\n\r\n\n  ");
        assert_eq!(replace_last_lf_group("\n\n\n  ", true), "\n\n  ");
    }

    #[test]
    fn accepts_all_20_expanded_stable_v5_10_0_valid_fixtures() {
        let fixture = upstream_fixture();
        let generated = &fixture["__generated"];
        assert_eq!(generated["version"], "5.10.0");
        assert_eq!(
            generated["sourceCommit"],
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(generated["inventory"]["logicalValid"], 12);
        assert_eq!(generated["inventory"]["logicalInvalid"], 19);
        assert_eq!(generated["inventory"]["valid"], 20);
        assert_eq!(generated["inventory"]["invalid"], 34);
        assert_eq!(generated["inventory"]["diagnostics"], 48);
        assert_eq!(generated["inventory"]["fixableInvalid"], 34);
        assert_eq!(generated["inventory"]["total"], 54);

        for (index, test) in fixture["valid"]
            .as_array()
            .expect("valid fixture array")
            .iter()
            .enumerate()
        {
            let source = test["code"].as_str().expect("valid fixture code");
            let filename = if test["parser"] == "typescript" {
                "fixture.tsx"
            } else {
                "fixture.jsx"
            };
            let diagnostics = run(source, Some(filename), test["options"].clone());
            assert!(
                diagnostics.is_empty(),
                "upstream valid fixture {index} reported {diagnostics:#?}:\n{source}"
            );
        }
    }

    #[test]
    fn reproduces_all_34_invalid_fixtures_with_exact_diagnostics_ranges_and_fixes() {
        let fixture = upstream_fixture();
        for (index, test) in fixture["invalid"]
            .as_array()
            .expect("invalid fixture array")
            .iter()
            .enumerate()
        {
            let source = test["code"].as_str().expect("invalid fixture code");
            let filename = if test["parser"] == "typescript" {
                "fixture.tsx"
            } else {
                "fixture.jsx"
            };
            let diagnostics = run(source, Some(filename), test["options"].clone());
            let expected = test["expectedDiagnostics"]
                .as_array()
                .expect("expected diagnostics");
            assert_eq!(
                diagnostics.len(),
                expected.len(),
                "invalid fixture {index} diagnostic count:\n{source}"
            );
            for (diagnostic_index, (actual, expected)) in
                diagnostics.iter().zip(expected).enumerate()
            {
                assert_eq!(
                    actual.message_id,
                    expected["messageId"].as_str().expect("message ID"),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.message,
                    expected["message"].as_str().expect("message"),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                let range = expected["range"].as_array().expect("expected range");
                assert_eq!(
                    actual.range,
                    TextRange::new(
                        range[0].as_u64().expect("range start") as u32,
                        range[1].as_u64().expect("range end") as u32,
                    ),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                let expected_fix = &expected["fix"];
                let actual_fix = &actual.suggestions[0].fixes[0];
                let fix_range = expected_fix["range"].as_array().expect("fix range");
                assert_eq!(
                    actual_fix.range,
                    TextRange::new(
                        fix_range[0].as_u64().expect("fix start") as u32,
                        fix_range[1].as_u64().expect("fix end") as u32,
                    ),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual_fix.replacement_text,
                    expected_fix["text"].as_str().expect("fix text"),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
            }
            assert_eq!(
                apply(source, &diagnostics),
                test["output"].as_str().expect("single-pass output"),
                "invalid fixture {index} output"
            );
        }
    }
}
