use oxc_ast::ast::{Expression, TaggedTemplateExpression};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regress::Regex;
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::{Diagnostic, DiagnosticDatum};

const REQUIRE_LOCALIZE_METADATA: &str = "require-localize-metadata";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CustomIdRequirement<'a> {
    Disabled,
    Present,
    Pattern(&'a str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RequireLocalizeMetadataOptions<'a> {
    require_description: bool,
    require_meaning: bool,
    require_custom_id: CustomIdRequirement<'a>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct LocalizeMetadata<'a> {
    meaning: Option<&'a str>,
    description: Option<&'a str>,
    custom_id: Option<&'a str>,
}

impl Scanner<'_> {
    pub(crate) fn check_require_localize_metadata(
        &mut self,
        tagged_template: &TaggedTemplateExpression<'_>,
    ) {
        if !self.options.is_enabled(REQUIRE_LOCALIZE_METADATA) {
            return;
        }
        let options = configured_options(&self.options.options);
        if !options.require_description
            && !options.require_meaning
            && options.require_custom_id == CustomIdRequirement::Disabled
        {
            return;
        }
        let Expression::Identifier(identifier) = tagged_template.tag.get_inner_expression() else {
            return;
        };
        if identifier.name != "$localize" {
            return;
        }
        let Some(template_element) = tagged_template.quasi.quasis.first() else {
            return;
        };
        let report_span = Span::new(
            template_element.span.start.saturating_sub(1),
            template_element.span.end + if template_element.tail { 1 } else { 2 },
        );
        let metadata = parse_metadata(template_element.value.raw.as_str().trim());

        if options.require_description
            && metadata
                .description
                .is_none_or(|description| description.is_empty())
        {
            self.report_require_localize_metadata(
                "requireLocalizeDescription",
                SmallVec::new(),
                report_span,
            );
        }
        if options.require_meaning && metadata.meaning.is_none_or(|meaning| meaning.is_empty()) {
            self.report_require_localize_metadata(
                "requireLocalizeMeaning",
                SmallVec::new(),
                report_span,
            );
        }

        let custom_id_matches = match options.require_custom_id {
            CustomIdRequirement::Disabled => true,
            CustomIdRequirement::Present => metadata.custom_id.is_some_and(|id| !id.is_empty()),
            CustomIdRequirement::Pattern(pattern) => metadata
                .custom_id
                .filter(|id| !id.is_empty())
                .is_some_and(|id| Regex::new(pattern).is_ok_and(|regex| regex.find(id).is_some())),
        };
        if !custom_id_matches {
            let mut data = SmallVec::new();
            let mut pattern_message = CompactString::new("");
            if let CustomIdRequirement::Pattern(pattern) = options.require_custom_id {
                pattern_message.push_str(" matching the pattern /");
                pattern_message.push_str(pattern);
                pattern_message.push_str("/ on '");
                pattern_message.push_str(metadata.custom_id.unwrap_or("undefined"));
                pattern_message.push('\'');
            }
            data.push(DiagnosticDatum {
                key: CompactString::from("patternMessage"),
                value: pattern_message,
            });
            self.report_require_localize_metadata("requireLocalizeCustomId", data, report_span);
        }
    }

    fn report_require_localize_metadata(
        &mut self,
        message_id: &'static str,
        data: SmallVec<[DiagnosticDatum; 2]>,
        span: Span,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name: REQUIRE_LOCALIZE_METADATA,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

fn configured_options(options: &Value) -> RequireLocalizeMetadataOptions<'_> {
    let option = options.as_array().and_then(|options| options.first());
    RequireLocalizeMetadataOptions {
        require_description: option
            .and_then(|option| option.get("requireDescription"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        require_meaning: option
            .and_then(|option| option.get("requireMeaning"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        require_custom_id: option
            .and_then(|option| option.get("requireCustomId"))
            .map_or(CustomIdRequirement::Disabled, |requirement| {
                if let Some(pattern) = requirement.as_str() {
                    CustomIdRequirement::Pattern(pattern)
                } else if requirement.as_bool() == Some(true) {
                    CustomIdRequirement::Present
                } else {
                    CustomIdRequirement::Disabled
                }
            }),
    }
}

fn parse_metadata(raw_text: &str) -> LocalizeMetadata<'_> {
    if !raw_text.starts_with(':') {
        return LocalizeMetadata::default();
    }
    let Some(end_of_block) = raw_text.rfind(':') else {
        return LocalizeMetadata::default();
    };
    let text = if end_of_block < 1 {
        ""
    } else {
        &raw_text[1..end_of_block]
    };
    let mut id_parts = text.split("@@");
    let meaning_and_description = id_parts.next().unwrap_or_default();
    let custom_id = id_parts.next();
    let mut meaning_parts = meaning_and_description.split('|');
    let first = meaning_parts.next().unwrap_or_default();
    let second = meaning_parts.next();
    let (meaning, description) = match second {
        Some(description) => (
            Some(first),
            (!description.is_empty()).then_some(description),
        ),
        None => (None, (!first.is_empty()).then_some(first)),
    };
    LocalizeMetadata {
        meaning,
        description,
        custom_id,
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "Pinned upstream fixtures use serde_json values and Vec assertions to mirror the JavaScript ABI."
)]
mod tests {
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use serde_json::{Value, json};

    use super::{LocalizeMetadata, REQUIRE_LOCALIZE_METADATA, parse_metadata};
    use crate::{Diagnostic, DiagnosticLoc, ScanOptions, scan_angular_eslint_with_options};

    const UPSTREAM_FIXTURE: &str = include_str!(
        "../../../npm/angular-eslint/test/fixtures/require-localize-metadata-v22.1.0.json"
    );

    fn scan(source: &str, options: Value) -> Vec<Diagnostic> {
        scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from(
                    REQUIRE_LOCALIZE_METADATA,
                )]),
                options,
            },
        )
        .into_vec()
    }

    #[test]
    fn replays_every_upstream_authored_valid_case() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid require-localize fixture");
        let valid = fixture["valid"]
            .as_array()
            .expect("fixture has valid cases");
        assert_eq!(valid.len(), 13);
        for test_case in valid {
            let diagnostics = scan(
                test_case["code"].as_str().expect("valid case has source"),
                test_case["options"].clone(),
            );
            assert!(
                diagnostics.is_empty(),
                "{}: {diagnostics:#?}",
                test_case["name"].as_str().expect("valid case has name"),
            );
        }
    }

    #[test]
    fn replays_every_upstream_authored_invalid_location_message_and_data() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid require-localize fixture");
        let invalid = fixture["invalid"]
            .as_array()
            .expect("fixture has invalid cases");
        assert_eq!(invalid.len(), 15);
        assert_eq!(
            invalid
                .iter()
                .map(|case| case["errors"].as_array().expect("case errors").len())
                .sum::<usize>(),
            16,
        );
        for test_case in invalid {
            let diagnostics = scan(
                test_case["code"].as_str().expect("invalid case has source"),
                test_case["options"].clone(),
            );
            let errors = test_case["errors"]
                .as_array()
                .expect("invalid case has errors");
            assert_eq!(
                diagnostics.len(),
                errors.len(),
                "{}: {diagnostics:#?}",
                test_case["name"].as_str().expect("invalid case has name"),
            );
            for (diagnostic, error) in diagnostics.iter().zip(errors) {
                assert_eq!(diagnostic.message_id, error["messageId"]);
                assert_eq!(
                    diagnostic.loc,
                    DiagnosticLoc {
                        start_line: error["line"].as_u64().expect("line") as u32,
                        start_column: error["column"].as_u64().expect("column") as u32 - 1,
                        end_line: error["endLine"].as_u64().expect("end line") as u32,
                        end_column: error["endColumn"].as_u64().expect("end column") as u32 - 1,
                    },
                    "{}",
                    test_case["name"].as_str().expect("invalid case has name"),
                );
                let actual_data = diagnostic
                    .data
                    .iter()
                    .map(|datum| (datum.key.as_str(), datum.value.as_str()))
                    .collect::<Vec<_>>();
                let expected_data = error["data"]
                    .as_object()
                    .expect("error data is an object")
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str().expect("string data")))
                    .collect::<Vec<_>>();
                assert_eq!(actual_data, expected_data);
            }
        }
    }

    #[test]
    fn parses_upstream_metadata_delimiters_and_empty_fields_exactly() {
        assert_eq!(parse_metadata("plain"), LocalizeMetadata::default());
        assert_eq!(
            parse_metadata(":description:message"),
            LocalizeMetadata {
                meaning: None,
                description: Some("description"),
                custom_id: None,
            },
        );
        assert_eq!(
            parse_metadata(":meaning|description@@id:message"),
            LocalizeMetadata {
                meaning: Some("meaning"),
                description: Some("description"),
                custom_id: Some("id"),
            },
        );
        assert_eq!(
            parse_metadata(":|description:message"),
            LocalizeMetadata {
                meaning: Some(""),
                description: Some("description"),
                custom_id: None,
            },
        );
        assert_eq!(
            parse_metadata(":@@id:message"),
            LocalizeMetadata {
                meaning: None,
                description: None,
                custom_id: Some("id"),
            },
        );
        assert_eq!(
            parse_metadata(":first|second|discarded@@id@@discarded:message"),
            LocalizeMetadata {
                meaning: Some("first"),
                description: Some("second"),
                custom_id: Some("id"),
            },
        );
    }

    #[test]
    fn defaults_to_no_requirements_and_forwards_each_option_independently() {
        let source = "$localize`Hello`;";
        assert!(scan(source, json!([])).is_empty());
        for (option, expected) in [
            (
                json!([{ "requireDescription": true }]),
                "requireLocalizeDescription",
            ),
            (
                json!([{ "requireMeaning": true }]),
                "requireLocalizeMeaning",
            ),
            (
                json!([{ "requireCustomId": true }]),
                "requireLocalizeCustomId",
            ),
        ] {
            let diagnostics = scan(source, option);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message_id, expected);
        }
    }

    #[test]
    fn reports_description_meaning_then_custom_id_in_upstream_order() {
        let diagnostics = scan(
            "$localize`Hello`;",
            json!([{
                "requireDescription": true,
                "requireMeaning": true,
                "requireCustomId": "^stable$"
            }]),
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            vec![
                "requireLocalizeDescription",
                "requireLocalizeMeaning",
                "requireLocalizeCustomId",
            ],
        );
        assert_eq!(
            diagnostics[2].data[0].value,
            " matching the pattern /^stable$/ on 'undefined'",
        );
    }

    #[test]
    fn checks_only_the_first_quasi_and_every_matching_tagged_template() {
        let diagnostics = scan(
            "$localize`:desc:${value}`;\n$localize`Hello`;\n$localize`:other:World`;",
            json!([{ "requireDescription": true }]),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].loc.start_line, 2);
    }

    #[test]
    fn accepts_identifier_tags_after_upstream_parenthesis_normalization() {
        let diagnostics = scan(
            "i18n.$localize`Hello`;\nother`Hello`;\n($localize)`Hello`;\n$localize`Hello`;",
            json!([{ "requireDescription": true }]),
        );
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.loc.start_line)
                .collect::<Vec<_>>(),
            vec![3, 4],
        );
    }

    #[test]
    fn trims_the_first_raw_quasi_and_uses_the_last_metadata_colon() {
        assert!(
            scan(
                "$localize`  :description:message  `;",
                json!([{ "requireDescription": true }]),
            )
            .is_empty(),
        );
        assert!(
            scan(
                "$localize`:meaning|description:message:with:colons`;",
                json!([{ "requireMeaning": true, "requireDescription": true }]),
            )
            .is_empty(),
        );
    }

    #[test]
    fn supports_pattern_matching_and_fails_closed_for_invalid_patterns() {
        assert!(
            scan(
                "$localize`:@@some.custom.id:Hello`;",
                json!([{ "requireCustomId": "^some.*id$" }]),
            )
            .is_empty(),
        );
        let mismatch = scan(
            "$localize`:@@some.custom.id:Hello`;",
            json!([{ "requireCustomId": "^wrong$" }]),
        );
        assert_eq!(mismatch.len(), 1);
        assert_eq!(
            mismatch[0].data[0].value,
            " matching the pattern /^wrong$/ on 'some.custom.id'",
        );
        assert_eq!(
            scan(
                "$localize`:@@some.custom.id:Hello`;",
                json!([{ "requireCustomId": "[" }]),
            )
            .len(),
            1,
        );
        assert!(
            scan(
                "$localize`:@@samesame.id:Hello`;",
                json!([{ "requireCustomId": "^(same)\\1\\.id$" }]),
            )
            .is_empty(),
        );
        assert!(
            scan(
                "$localize`:@@some.custom.id:Hello`;",
                json!([{ "requireCustomId": "^(?=some)some.*id$" }]),
            )
            .is_empty(),
        );
    }

    #[test]
    fn preserves_utf16_columns_and_template_element_locations() {
        let diagnostics = scan(
            "const marker = '😀'; $localize`Hello ${name}`;",
            json!([{ "requireMeaning": true }]),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].loc,
            DiagnosticLoc {
                start_line: 1,
                start_column: 30,
                end_line: 1,
                end_column: 39,
            },
        );
    }

    #[test]
    fn ignores_strings_comments_and_malformed_typescript() {
        let source = r#"
const text = "$localize`Hello`";
// $localize`Hello`;
const tagged = $localize`:description:Hello`;
"#;
        assert!(scan(source, json!([{ "requireDescription": true }])).is_empty());
        assert!(
            scan(
                "const tagged = $localize`unterminated",
                json!([{ "requireDescription": true }]),
            )
            .is_empty(),
        );
    }
}
