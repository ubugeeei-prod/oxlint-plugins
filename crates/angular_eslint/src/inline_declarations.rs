use compact_str::ToCompactString;
use oxc_ast::ast::{ArrayExpressionElement, Class, Expression, ObjectPropertyKind, PropertyKey};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::{Diagnostic, DiagnosticDatum};

const RULE_NAME: &str = "component-max-inline-declarations";
const MESSAGE_ID: &str = "componentMaxInlineDeclarations";
const DEFAULT_TEMPLATE_LIMIT: f64 = 3.0;
const DEFAULT_STYLES_LIMIT: f64 = 3.0;
const DEFAULT_ANIMATIONS_LIMIT: f64 = 15.0;

#[derive(Clone, Debug)]
struct Limit {
    value: f64,
    display: CompactString,
}

impl Limit {
    fn default(value: f64) -> Self {
        Self {
            value,
            display: (value as usize).to_compact_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct Limits {
    template: Limit,
    styles: Limit,
    animations: Limit,
}

impl Scanner<'_> {
    pub(crate) fn check_component_inline_declarations(&mut self, class: &Class<'_>) {
        if !self.options.is_enabled(RULE_NAME) {
            return;
        }
        let limits = Limits::from_options(&self.options.options);
        for decorator in &class.decorators {
            let Expression::CallExpression(call) = decorator.expression.get_inner_expression()
            else {
                continue;
            };
            let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
                continue;
            };
            if callee.name != "Component" {
                continue;
            }
            let Some(Expression::ObjectExpression(metadata)) = call
                .arguments
                .first()
                .and_then(|argument| argument.as_expression())
                .map(Expression::get_inner_expression)
            else {
                continue;
            };
            for property in &metadata.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    continue;
                };
                match property_name(&property.key) {
                    Some("template") => {
                        let line_count = static_lines(self.source_text, &property.value);
                        self.report_inline_limit(
                            "template",
                            line_count,
                            &limits.template,
                            property.value.span(),
                        );
                    }
                    Some("styles") => {
                        let line_count = style_lines(self.source_text, &property.value);
                        self.report_inline_limit(
                            "styles",
                            line_count,
                            &limits.styles,
                            property.value.span(),
                        );
                    }
                    Some("animations") => {
                        let Some(line_count) = animation_lines(self.source_text, &property.value)
                        else {
                            continue;
                        };
                        self.report_inline_limit(
                            "animations",
                            line_count,
                            &limits.animations,
                            property.value.span(),
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    fn report_inline_limit(
        &mut self,
        property_type: &'static str,
        line_count: usize,
        limit: &Limit,
        span: Span,
    ) {
        if line_count as f64 <= limit.value {
            return;
        }
        let mut data = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("propertyType"),
            value: CompactString::from(property_type),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("lineCount"),
            value: line_count.to_compact_string(),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("max"),
            value: limit.display.clone(),
        });
        self.diagnostics.push(Diagnostic {
            rule_name: RULE_NAME,
            message_id: MESSAGE_ID,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

impl Limits {
    fn from_options(options: &Value) -> Self {
        let object = options
            .as_array()
            .and_then(|options| options.first())
            .and_then(Value::as_object);
        Self {
            template: configured_limit(object.and_then(|value| value.get("template")))
                .unwrap_or_else(|| Limit::default(DEFAULT_TEMPLATE_LIMIT)),
            styles: configured_limit(object.and_then(|value| value.get("styles")))
                .unwrap_or_else(|| Limit::default(DEFAULT_STYLES_LIMIT)),
            animations: configured_limit(object.and_then(|value| value.get("animations")))
                .unwrap_or_else(|| Limit::default(DEFAULT_ANIMATIONS_LIMIT)),
        }
    }
}

fn configured_limit(value: Option<&Value>) -> Option<Limit> {
    let value = value?;
    let number = value.as_f64()?;
    if !number.is_finite() {
        return None;
    }
    Some(Limit {
        value: number,
        display: value.to_compact_string(),
    })
}

fn property_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn static_lines(source_text: &str, expression: &Expression<'_>) -> usize {
    match expression {
        Expression::TemplateLiteral(template) => template
            .quasis
            .first()
            .map_or(0, |quasi| split_line_count(quasi.value.raw.as_str())),
        Expression::StringLiteral(literal) => source_text
            .get(literal.span.start as usize..literal.span.end as usize)
            .map_or(0, split_line_count),
        _ => 0,
    }
}

fn style_lines(source_text: &str, expression: &Expression<'_>) -> usize {
    let Expression::ArrayExpression(array) = expression else {
        return static_lines(source_text, expression);
    };
    array
        .elements
        .iter()
        .map(|element| match element {
            ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => 0,
            _ => element
                .as_expression()
                .map_or(0, |expression| static_lines(source_text, expression)),
        })
        .sum()
}

fn animation_lines(source_text: &str, expression: &Expression<'_>) -> Option<usize> {
    let Expression::ArrayExpression(array) = expression else {
        return None;
    };
    if array.elements.is_empty() {
        return None;
    }
    Some(
        count_line_breaks(
            source_text
                .get(array.span.start as usize..array.span.end as usize)
                .unwrap_or(""),
        )
        .saturating_sub(2)
        .max(1),
    )
}

fn split_line_count(value: &str) -> usize {
    let value = value.trim();
    let bytes = value.as_bytes();
    let mut lines = 1;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                lines += 1;
                index += usize::from(bytes.get(index + 1) == Some(&b'\n')) + 1;
            }
            b'\n' => {
                lines += 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    lines
}

fn count_line_breaks(value: &str) -> usize {
    let mut count = 0;
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\r' => {
                count += 1;
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            '\n' | '\u{2028}' | '\u{2029}' => count += 1,
            _ => {}
        }
    }
    count
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "Pinned upstream option cases use serde_json::json and Vec-shaped assertions to mirror the JavaScript ABI exactly."
)]
mod tests {
    use compact_str::ToCompactString;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use serde_json::{Value, json};

    use super::{MESSAGE_ID, RULE_NAME};
    use crate::{Diagnostic, ScanOptions, scan_angular_eslint_with_options};

    fn scan(source: &str, options: Value) -> Vec<Diagnostic> {
        scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from(RULE_NAME)]),
                options,
            },
        )
        .into_vec()
    }

    fn datum<'a>(diagnostic: &'a Diagnostic, key: &str) -> Option<&'a str> {
        diagnostic
            .data
            .iter()
            .find(|datum| datum.key == key)
            .map(|datum| datum.value.as_str())
    }

    fn assert_diagnostic(
        diagnostic: &Diagnostic,
        property_type: &str,
        line_count: usize,
        max: &str,
    ) {
        assert_eq!(diagnostic.rule_name, RULE_NAME);
        assert_eq!(diagnostic.message_id, MESSAGE_ID);
        assert_eq!(datum(diagnostic, "propertyType"), Some(property_type));
        assert_eq!(
            datum(diagnostic, "lineCount"),
            Some(line_count.to_compact_string().as_str())
        );
        assert_eq!(datum(diagnostic, "max"), Some(max));
    }

    // Pinned from angular-eslint v22.0.0, commit
    // 7ee4556badebf8c140ffdefdd0b07b02820d5e96.
    #[test]
    fn ports_every_upstream_valid_case() {
        for source in [
            r#"@Component({ template: '<div>just one line template</div>' }) class Test {}"#,
            r#"@Component({ styles: ['div { display: none; }'] }) class Test {}"#,
            r#"@Component({ styles: 'div { display: none; }' }) class Test {}"#,
            r#"@Component({ animations: [state('void', style({opacity: 0, transform: 'scale(1, 0)'}))] }) class Test {}"#,
            r#"@Component({ styleUrls: ['./foobar.scss'], templateUrl: './foobar.html' }) class Test {}"#,
            r#"@Component({
                animations: [
                    state('void', style({opacity: 0, transform: 'scale(1, 0)'}))
                ],
                templateUrl: './foobar.html',
            }) class Test {}"#,
        ] {
            assert!(scan(source, Value::Null).is_empty(), "{source}");
        }
    }

    #[test]
    fn ports_upstream_template_invalid_cases() {
        let default = scan(
            r#"@Component({
                template: `
                    <div>first line</div>
                    <div>second line</div>
                    <div>third line</div>
                    <div>fourth line</div>
                `
            }) class Test {}"#,
            Value::Null,
        );
        assert_eq!(default.len(), 1);
        assert_diagnostic(&default[0], "template", 4, "3");

        let custom = scan(
            r#"@Component({ template: '<div>first line</div>' }) class Test {}"#,
            json!([{"template": 0}]),
        );
        assert_eq!(custom.len(), 1);
        assert_diagnostic(&custom[0], "template", 1, "0");
    }

    #[test]
    fn ports_upstream_styles_invalid_cases() {
        let cases = [
            (
                r#"@Component({
                    styles: [
                        `
                            div {
                                display: block;
                                height: 40px;
                            }
                        `
                    ]
                }) class Test {}"#,
                Value::Null,
                4,
                "3",
            ),
            (
                r#"@Component({
                    styles: `
                        div {
                            display: block;
                            height: 40px;
                        }
                    `
                }) class Test {}"#,
                Value::Null,
                4,
                "3",
            ),
            (
                r#"@Component({
                    styles: [
                        `
                            div {
                                display: block;
                            }
                        `,
                        `
                            span {
                                width: 30px;
                            }
                        `
                    ]
                }) class Test {}"#,
                Value::Null,
                6,
                "3",
            ),
            (
                r#"@Component({ styles: ['div { display: none; }'] }) class Test {}"#,
                json!([{"styles": 0}]),
                1,
                "0",
            ),
        ];
        for (source, options, line_count, max) in cases {
            let diagnostics = scan(source, options);
            assert_eq!(diagnostics.len(), 1, "{source}");
            assert_diagnostic(&diagnostics[0], "styles", line_count, max);
        }
    }

    #[test]
    fn ports_upstream_animations_invalid_cases() {
        let cases = [
            (
                r#"@Component({
                    animations: [{

                        transformPanelWrap: trigger('transformPanelWrap', [
                            transition('* => void', query('@transformPanel', [animateChild()], {optional: true})),
                        ]),
                        transformPanel: trigger('transformPanel', [
                            state('void', style({
                                transform: 'scaleY(0.8)',
                                minWidth: '100%',
                                opacity: 0
                            })),
                            state('showing', style({
                                opacity: 1,
                                minWidth: 'calc(100% + 32px)',
                                transform: 'scaleY(1)'
                            })),
                            state('next', style({height: '0px', visibility: 'hidden'}))
                        ])
                    }]
                }) class Test {}"#,
                Value::Null,
                16,
                "15",
            ),
            (
                r#"@Component({
                    animations: [

                        trigger('dialogContainer', [
                            transition('* => void', query('@transformPanel', [animateChild()], {optional: true}))
                        ]),
                        trigger('transformPanel', [
                            state('void', style({
                                transform: 'scaleY(0.8)',
                                minWidth: '100%',
                                opacity: 0
                            })),
                            state('showing', style({
                                opacity: 1,
                                minWidth: 'calc(100% + 32px)',
                                transform: 'scaleY(1)'
                            }))
                        ]),
                        trigger('transformPanel', [
                            state('void', style({opacity: 0, transform: 'scale(1, 0)'}))
                        ])
                    ]
                }) class Test {}"#,
                Value::Null,
                18,
                "15",
            ),
            (
                r#"@Component({
                    animations: [{

                        transformPanel: trigger('transformPanel', [
                            state('void', style({opacity: 0, transform: 'scale(1, 0)'}))
                        ])
                    }]
                }) class Test {}"#,
                json!([{"animations": 2}]),
                3,
                "2",
            ),
        ];
        for (source, options, line_count, max) in cases {
            let diagnostics = scan(source, options);
            assert_eq!(diagnostics.len(), 1, "{source}");
            assert_diagnostic(&diagnostics[0], "animations", line_count, max);
        }
    }

    #[test]
    fn honors_independent_fractional_and_partial_limits() {
        let source = r#"@Component({
            template: `one
                two`,
            styles: [`one
                two`],
            animations: [
                one(),
                two(),
                three()
            ]
        }) class Test {}"#;
        let diagnostics = scan(
            source,
            json!([{"template": 1.5, "styles": 2, "animations": 1}]),
        );
        assert_eq!(diagnostics.len(), 2);
        assert_diagnostic(&diagnostics[0], "template", 2, "1.5");
        assert_diagnostic(&diagnostics[1], "animations", 2, "1");
    }

    #[test]
    fn uses_defaults_for_omitted_or_non_numeric_fields() {
        let source = r#"@Component({
            template: `one
                two
                three
                four`,
            styles: `one
                two
                three
                four`
        }) class Test {}"#;
        let diagnostics = scan(source, json!([{"template": "invalid"}]));
        assert_eq!(diagnostics.len(), 2);
        assert_diagnostic(&diagnostics[0], "template", 4, "3");
        assert_diagnostic(&diagnostics[1], "styles", 4, "3");
    }

    #[test]
    fn ignores_dynamic_values_holes_spreads_and_empty_animations() {
        for source in [
            r#"@Component({ template }) class Test {}"#,
            r#"@Component({ template: `${first}
                second
                third
                fourth` }) class Test {}"#,
            r#"@Component({ styles: [styles, , ...moreStyles] }) class Test {}"#,
            r#"@Component({ animations: animations }) class Test {}"#,
            r#"@Component({ animations: [] }) class Test {}"#,
        ] {
            assert!(
                scan(
                    source,
                    json!([{"template": 1, "styles": 0, "animations": 0}])
                )
                .is_empty()
            );
        }
    }

    #[test]
    fn only_scans_component_metadata_and_reports_duplicate_properties() {
        assert!(
            scan(
                r#"@Directive({ template: `a
b`, styles: [`a
b`] }) class Test {}"#,
                json!([{"template": 0, "styles": 0}])
            )
            .is_empty()
        );
        let diagnostics = scan(
            r#"@Other() @Component({
                template: 'one',
                template: 'two',
                styles: ['three']
            }) class Test {}"#,
            json!([{"template": 0, "styles": 0}]),
        );
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(datum(&diagnostics[0], "propertyType"), Some("template"));
        assert_eq!(datum(&diagnostics[1], "propertyType"), Some("template"));
        assert_eq!(datum(&diagnostics[2], "propertyType"), Some("styles"));
    }

    #[test]
    fn matches_upstream_property_key_selector_semantics() {
        let diagnostics = scan(
            r#"@Component({
                [template]: 'computed identifier',
                "template": 'quoted',
                ["styles"]: ['computed literal'],
                [`animations`]: [computedLiteral()]
            }) class Test {}"#,
            json!([{"template": 0, "styles": 0, "animations": 0}]),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_diagnostic(&diagnostics[0], "template", 1, "0");
    }

    #[test]
    fn preserves_exact_value_locations_and_utf16_columns() {
        let source = "'😀'; @Component({ template: `one\n two`, styles: ['one'] }) class Test {}";
        let diagnostics = scan(source, json!([{"template": 1, "styles": 0}]));
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].loc.start_line, 1);
        assert_eq!(diagnostics[0].loc.start_column, 29);
        assert_eq!(diagnostics[0].loc.end_line, 2);
        assert_eq!(diagnostics[1].loc.start_line, 2);
    }

    #[test]
    fn counts_crlf_and_cr_template_lines_like_upstream() {
        for source in [
            "@Component({ template: `one\r\ntwo\r\nthree\r\nfour` }) class Test {}",
            "@Component({ template: `one\rtwo\rthree\rfour` }) class Test {}",
        ] {
            let diagnostics = scan(source, Value::Null);
            assert_eq!(diagnostics.len(), 1);
            assert_diagnostic(&diagnostics[0], "template", 4, "3");
        }
    }

    #[test]
    fn rule_selection_and_parse_errors_fail_closed() {
        assert!(
            scan_angular_eslint_with_options(
                "@Component({ template: `a\nb\nc\nd` }) class Test {}",
                "fixture.ts",
                &ScanOptions {
                    rule_names: SmallVec::from_vec(vec![CompactString::from("pipe-prefix")]),
                    options: Value::Null,
                },
            )
            .is_empty()
        );
        assert!(scan("@Component({ template: `unterminated })", Value::Null).is_empty());
    }
}
