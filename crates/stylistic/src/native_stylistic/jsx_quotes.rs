//! `@stylistic/jsx-quotes` implemented against Oxc's JSX AST.
//!
//! A string literal has the same lexical shape inside and outside JSX, and
//! TypeScript's angle-bracket syntax overlaps JSX opening tags. Parsing only
//! when this JSX-specific rule is enabled keeps the common token-rule path
//! allocation-light while giving this rule the same attribute boundary and
//! decoded entity semantics as upstream.

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXAttribute, JSXAttributeValue};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::Value;

use crate::LintDiagnostic;

use super::helpers::{ReplacementDiagnostic, option_str, push_replacement_diagnostic};

const RULE_NAME: &str = "jsx-quotes";

#[derive(Clone, Copy, PartialEq, Eq)]
enum QuotePreference {
    Double,
    Single,
}

impl QuotePreference {
    fn from_options(options: &Value) -> Self {
        if option_str(options, 0) == Some("prefer-single") {
            QuotePreference::Single
        } else {
            QuotePreference::Double
        }
    }

    const fn quote(self) -> char {
        match self {
            QuotePreference::Double => '"',
            QuotePreference::Single => '\'',
        }
    }

    const fn unexpected_message(self) -> &'static str {
        match self {
            QuotePreference::Double => "Unexpected usage of singlequote.",
            QuotePreference::Single => "Unexpected usage of doublequote.",
        }
    }
}

pub(crate) fn check_jsx_quotes(
    source_text: &str,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allocator = Allocator::default();
    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        let parsed = Parser::new(&allocator, source_text, source_type).parse();
        if parsed.errors.is_empty() {
            let mut checker = JsxQuotesChecker {
                source_text,
                preference: QuotePreference::from_options(options),
                diagnostics,
            };
            checker.visit_program(&parsed.program);
            return;
        }
    }
}

struct JsxQuotesChecker<'a, 'd> {
    source_text: &'a str,
    preference: QuotePreference,
    diagnostics: &'d mut Vec<LintDiagnostic>,
}

impl<'a> Visit<'a> for JsxQuotesChecker<'a, '_> {
    fn visit_jsx_attribute(&mut self, attribute: &JSXAttribute<'a>) {
        if let Some(JSXAttributeValue::StringLiteral(literal)) = &attribute.value {
            let start = literal.span.start as usize;
            let end = literal.span.end as usize;
            let raw = &self.source_text[start..end];
            let expected = self.preference.quote();

            // Upstream allows the alternate delimiter when the decoded value
            // contains the preferred quote, because JSX attribute strings do
            // not support escaping their delimiter. `literal.value` includes
            // decoded named and numeric character references.
            let already_preferred = raw.starts_with(expected) && raw.ends_with(expected);
            if !already_preferred
                && !literal.value.as_str().contains(expected)
                && !contains_quote_character_reference(raw, expected)
            {
                let replacement = match self.preference {
                    QuotePreference::Double => raw.replace('\'', "\""),
                    QuotePreference::Single => raw.replace('"', "'"),
                };
                push_replacement_diagnostic(
                    self.diagnostics,
                    ReplacementDiagnostic {
                        rule_name: RULE_NAME,
                        message_id: "unexpected",
                        message: self.preference.unexpected_message(),
                        start,
                        end,
                        suggestion_id: "fixQuote",
                        suggestion_message: "Convert JSX attribute quote style.",
                    },
                    replacement,
                );
            }
        }

        walk::walk_jsx_attribute(self, attribute);
    }
}

/// Oxc currently preserves JSX character references in `StringLiteral::value`,
/// while Espree (used by upstream's test runner) decodes them. Recognizing only
/// the two quote code points here reproduces the upstream decision without
/// paying for a general HTML entity decoder.
fn contains_quote_character_reference(raw: &str, expected: char) -> bool {
    let named = match expected {
        '"' => "&quot;",
        '\'' => "&apos;",
        _ => return false,
    };
    if raw.contains(named) {
        return true;
    }

    let expected_code_point = expected as u32;
    let bytes = raw.as_bytes();
    let mut cursor = 0_usize;
    while cursor + 3 < bytes.len() {
        let Some(relative) = raw[cursor..].find("&#") else {
            break;
        };
        let start = cursor + relative + 2;
        let hexadecimal = matches!(bytes.get(start), Some(b'x' | b'X'));
        let digits_start = start + usize::from(hexadecimal);
        let Some(relative_end) = raw[digits_start..].find(';') else {
            break;
        };
        let digits_end = digits_start + relative_end;
        let radix = if hexadecimal { 16 } else { 10 };
        if digits_start < digits_end
            && u32::from_str_radix(&raw[digits_start..digits_end], radix)
                .is_ok_and(|value| value == expected_code_point)
        {
            return true;
        }
        cursor = digits_end + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextRange;

    fn run(source: &str, option: Option<&str>) -> Vec<LintDiagnostic> {
        let options = option.map_or(Value::Null, |value| {
            Value::Array(std::iter::once(Value::String(value.to_owned())).collect())
        });
        let mut diagnostics = Vec::new();
        check_jsx_quotes(source, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(diagnostics: &[LintDiagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id.as_str())
            .collect()
    }

    fn fixes(diagnostics: &[LintDiagnostic]) -> Vec<(TextRange, &str)> {
        diagnostics
            .iter()
            .map(|diagnostic| {
                let fix = &diagnostic.suggestions[0].fixes[0];
                (fix.range, fix.replacement_text.as_str())
            })
            .collect()
    }

    #[test]
    fn accepts_every_upstream_valid_case() {
        let cases = [
            ("<foo bar=\"baz\" />", None),
            ("<foo bar='\"' />", None),
            ("<foo bar=\"'\" />", Some("prefer-single")),
            ("<foo bar='baz' />", Some("prefer-single")),
            ("<foo bar=\"baz\">\"</foo>", None),
            ("<foo bar='baz'>'</foo>", Some("prefer-single")),
            ("<foo bar={'baz'} />", None),
            ("<foo bar={\"baz\"} />", Some("prefer-single")),
            ("<foo bar={baz} />", None),
            ("<foo bar />", None),
            ("<foo bar='&quot;' />", Some("prefer-single")),
            ("<foo bar=\"&quot;\" />", None),
            ("<foo bar='&#39;' />", Some("prefer-single")),
            ("<foo bar=\"&#39;\" />", None),
        ];

        for (source, option) in cases {
            assert!(
                run(source, option).is_empty(),
                "upstream valid case was rejected: {source}"
            );
        }
    }

    #[test]
    fn ports_every_upstream_invalid_case_with_exact_ranges_and_fixes() {
        let cases = [
            (
                "<foo bar='baz' />",
                None,
                TextRange::new(9, 14),
                "<foo bar=\"baz\" />",
                "Unexpected usage of singlequote.",
            ),
            (
                "<foo bar=\"baz\" />",
                Some("prefer-single"),
                TextRange::new(9, 14),
                "<foo bar='baz' />",
                "Unexpected usage of doublequote.",
            ),
            (
                "<foo bar=\"&quot;\" />",
                Some("prefer-single"),
                TextRange::new(9, 17),
                "<foo bar='&quot;' />",
                "Unexpected usage of doublequote.",
            ),
            (
                "<foo bar='&#39;' />",
                None,
                TextRange::new(9, 16),
                "<foo bar=\"&#39;\" />",
                "Unexpected usage of singlequote.",
            ),
        ];

        for (source, option, range, output, message) in cases {
            let diagnostics = run(source, option);
            assert_eq!(ids(&diagnostics), ["unexpected"], "source: {source}");
            assert_eq!(diagnostics[0].range, range, "source: {source}");
            assert_eq!(diagnostics[0].message, message, "source: {source}");
            assert_eq!(
                fixes(&diagnostics),
                [(range, &output[range.start as usize..range.end as usize])],
                "source: {source}"
            );
        }
    }

    #[test]
    fn allows_alternate_delimiter_when_decoded_value_contains_preferred_quote() {
        for source in [
            "<foo literal='\"' />",
            "<foo named='&quot;' />",
            "<foo decimal='&#34;' />",
            "<foo hexadecimal='&#x22;' />",
        ] {
            assert!(run(source, None).is_empty(), "rejected {source}");
        }

        for source in [
            "<foo literal=\"'\" />",
            "<foo named=\"&apos;\" />",
            "<foo decimal=\"&#39;\" />",
            "<foo hexadecimal=\"&#x27;\" />",
        ] {
            assert!(
                run(source, Some("prefer-single")).is_empty(),
                "rejected {source}"
            );
        }
    }

    #[test]
    fn reports_multiple_nested_member_namespaced_and_hyphenated_attributes() {
        let source = concat!(
            "<UI.Root data-id='root'>",
            "<svg:path xml:lang='en' aria-label='label' />",
            "<Child enabled title='child' />",
            "</UI.Root>"
        );
        let diagnostics = run(source, None);

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| &source
                    [diagnostic.range.start as usize..diagnostic.range.end as usize])
                .collect::<Vec<_>>(),
            ["'root'", "'en'", "'label'", "'child'"]
        );
        assert_eq!(
            fixes(&diagnostics)
                .iter()
                .map(|(_, replacement)| *replacement)
                .collect::<Vec<_>>(),
            ["\"root\"", "\"en\"", "\"label\"", "\"child\""]
        );
    }

    #[test]
    fn ignores_non_attribute_strings_and_expression_containers() {
        let source = concat!(
            "import value from 'module';",
            "const plain = 'string';",
            "const object = {'key': 'value'};",
            "const template = `<App title='text' />`;",
            "const regexp = /<App title='text' \\/>/;",
            "const element = <App ",
            "fromExpression={'value'} ",
            "nested={{key: 'value'}} ",
            "child={<Child title='nested' />} ",
            ">text 'child'<span>{'expression child'}</span></App>;"
        );
        let diagnostics = run(source, None);

        assert_eq!(ids(&diagnostics), ["unexpected"]);
        let range = diagnostics[0].range;
        assert_eq!(
            &source[range.start as usize..range.end as usize],
            "'nested'"
        );
    }

    #[test]
    fn ignores_typescript_angle_brackets_generics_and_type_literals() {
        for source in [
            "type Box<T = 'default'> = { value: 'literal' };",
            "interface Box<T = 'default'> { value: 'literal' }",
            "function identity<T = 'default'>(value: T): T { return value; }",
            "const identity = <T extends 'default'>(value: T) => value;",
            "const result = left < 'middle' && right > 'floor';",
            "const asserted = value as 'literal';",
        ] {
            assert!(run(source, None).is_empty(), "false positive for {source}");
        }
    }

    #[test]
    fn keeps_jsx_children_boolean_spreads_and_nested_expression_assignments_out_of_scope() {
        let source = concat!(
            "<>",
            "<App enabled {...props} ",
            "value={fallback = 'next'} ",
            "compare={left <= 'middle' && right >= 'floor'} ",
            "nested={{ key: assigned = 'value' }}>",
            "'plain JSX text'",
            "</App>",
            "</>"
        );
        assert!(run(source, None).is_empty());
    }

    #[test]
    fn preserves_byte_ranges_for_multiline_unicode_attributes() {
        let source = "<日本語\n  ラベル='値'\n  emoji='😀'\n/>";
        let diagnostics = run(source, None);

        assert_eq!(ids(&diagnostics), ["unexpected", "unexpected"]);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| &source
                    [diagnostic.range.start as usize..diagnostic.range.end as usize])
                .collect::<Vec<_>>(),
            ["'値'", "'😀'"]
        );
        assert_eq!(
            fixes(&diagnostics)
                .iter()
                .map(|(_, replacement)| *replacement)
                .collect::<Vec<_>>(),
            ["\"値\"", "\"😀\""]
        );
    }

    #[test]
    fn defaults_to_prefer_double_for_empty_and_unrecognized_options() {
        assert_eq!(ids(&run("<App title='value' />", None)), ["unexpected"]);
        assert_eq!(
            ids(&run("<App title='value' />", Some("unsupported"))),
            ["unexpected"]
        );
        assert!(run("<App title=\"value\" />", None).is_empty());
    }

    #[test]
    fn falls_back_to_javascript_jsx_script_grammar() {
        let source = "with (scope) { node = <App title='value' />; }";
        let diagnostics = run(source, None);
        assert_eq!(ids(&diagnostics), ["unexpected"]);
        assert_eq!(
            &source[diagnostics[0].range.start as usize..diagnostics[0].range.end as usize],
            "'value'"
        );
    }

    #[test]
    fn supports_uppercase_hexadecimal_quote_references() {
        assert!(run("<App title='&#X22;' />", None).is_empty());
        assert!(run("<App title=\"&#X27;\" />", Some("prefer-single")).is_empty());
    }

    #[test]
    fn does_not_report_malformed_programs() {
        for source in [
            "<App title='value'",
            "<App title='value></App>",
            "const element = <App title='value' /> + ;",
            "const unterminated = 'value",
        ] {
            assert!(run(source, None).is_empty(), "reported malformed {source}");
        }
    }
}
